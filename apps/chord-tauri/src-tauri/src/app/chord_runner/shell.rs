use crate::app::chord_runner::ChordActionTask;
use crate::models::ShellChordAction;
use crate::process::{COMMAND_OUTPUT_LIMIT, COMMAND_TIMEOUT, bounded_output};
use anyhow::{Context, Result, bail};
use nject::injectable;
use std::fmt::Write;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;
use tokio::sync::Semaphore;

const MAX_CONCURRENT_SHELL_COMMANDS: usize = 16;
static SHELL_COMMAND_SLOTS: Semaphore = Semaphore::const_new(MAX_CONCURRENT_SHELL_COMMANDS);
/// How much of each output stream the log keeps. The full output is still drained and bounded
/// by `COMMAND_OUTPUT_LIMIT`.
const LOGGED_OUTPUT_BYTES: usize = 4 * 1024;
/// The Hint Panel shows two lines of an error, so a failure carries one line of its output.
const ERROR_OUTPUT_BYTES: usize = 200;

#[injectable]
pub struct ShellChordActionTaskRunner {
    handle: AppHandle,
}

#[derive(Debug)]
pub struct ShellChordActionTaskRun {
    join_handle: JoinHandle<Result<()>>,
}

impl ShellChordActionTaskRunner {
    pub fn start(
        &self,
        task: &ChordActionTask,
        action: &ShellChordAction,
    ) -> Result<ShellChordActionTaskRun> {
        let command = action.command.clone();
        let num_times = task.num_times;
        let join_handle = tauri::async_runtime::spawn(async move {
            for _ in 0..num_times {
                run_shell_command(&command, None).await?;
            }
            Ok(())
        });

        Ok(ShellChordActionTaskRun { join_handle })
    }

    pub async fn end(&self, task_run: &mut ShellChordActionTaskRun) -> Result<()> {
        (&mut task_run.join_handle).await??;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn abort(&self, task_run: ShellChordActionTaskRun) -> Result<()> {
        task_run.join_handle.abort();
        Ok(())
    }
}

pub(crate) async fn run_shell_command(shell: &str, current_dir: Option<PathBuf>) -> Result<()> {
    let _permit = SHELL_COMMAND_SLOTS
        .try_acquire()
        .context("too many shell commands are already running")?;
    let mut command = crate::process::shell_command(shell);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    log::debug!("Running shell command: {:?}", command);

    let started = Instant::now();
    let output = bounded_output(&mut command, COMMAND_OUTPUT_LIMIT, COMMAND_TIMEOUT)
        .await
        .with_context(|| format!("failed to run shell command `{shell}`"))?;
    let stdout = output.stdout.to_lossy_text();
    let stderr = output.stderr.to_lossy_text();
    log_shell_output(
        shell,
        output.status,
        stdout.trim(),
        stderr.trim(),
        started.elapsed(),
    )
}

/// A command that exits 0 but prints something often did not do what the chord meant (a CLI
/// that moved a subcommand, say), so its output is logged at the default level. Failures log
/// their full output; the returned error carries one line of it for the Hint Panel, and the
/// caller logs that error with the chord that ran the command.
fn log_shell_output(
    shell: &str,
    status: ExitStatus,
    stdout: &str,
    stderr: &str,
    elapsed: Duration,
) -> Result<()> {
    let status_text = describe_exit_status(status);
    let elapsed_ms = elapsed.as_millis();
    let mut streams = String::new();
    for (name, text) in [("stdout", stdout), ("stderr", stderr)] {
        if !text.is_empty() {
            let _ = write!(streams, "\n{name}: {}", excerpt(text, LOGGED_OUTPUT_BYTES));
        }
    }

    if status.success() {
        if streams.is_empty() {
            log::debug!("Shell command exited with {status_text} after {elapsed_ms} ms: `{shell}`");
        } else {
            log::info!(
                "Shell command exited with {status_text} after {elapsed_ms} ms: `{shell}`{streams}"
            );
        }
        return Ok(());
    }

    log::warn!("Shell command exited with {status_text} after {elapsed_ms} ms: `{shell}`{streams}");
    match first_line(stderr).or_else(|| first_line(stdout)) {
        Some(line) => bail!(
            "`{shell}` exited with {status_text}: {}",
            excerpt(line, ERROR_OUTPUT_BYTES)
        ),
        None => bail!("`{shell}` exited with {status_text} and printed nothing"),
    }
}

fn describe_exit_status(status: ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("code {code}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return format!("signal {signal}");
        }
    }
    status.to_string()
}

fn first_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

/// `text` cut to at most `limit` bytes on a character boundary, marked when cut.
fn excerpt(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [{} more bytes]", &text[..end], text.len() - end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(unix)]
    #[test]
    fn failures_carry_the_first_line_of_stderr() {
        let error = log_shell_output(
            "rift-cli nope",
            exit_status(2),
            "",
            "\nerror: unrecognized subcommand 'nope'\n\nUsage: rift-cli <COMMAND>",
            Duration::from_millis(3),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "`rift-cli nope` exited with code 2: error: unrecognized subcommand 'nope'"
        );
    }

    #[cfg(unix)]
    #[test]
    fn failures_fall_back_to_stdout_then_to_saying_nothing_was_printed() {
        let error = log_shell_output("false", exit_status(1), "", "", Duration::ZERO).unwrap_err();
        assert_eq!(
            error.to_string(),
            "`false` exited with code 1 and printed nothing"
        );

        let error =
            log_shell_output("check", exit_status(1), "not ready", "", Duration::ZERO).unwrap_err();
        assert_eq!(error.to_string(), "`check` exited with code 1: not ready");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_missing_command_explains_itself() {
        let error = run_shell_command("chord-test-missing-command-7f3a", None)
            .await
            .unwrap_err();
        let message = error.to_string();

        assert!(message.contains("exited with code 127"), "{message}");
        assert!(message.contains("not found"), "{message}");
    }

    #[test]
    fn excerpts_cut_on_a_character_boundary() {
        assert_eq!(excerpt("short", 10), "short");
        assert_eq!(excerpt("ab🧠cd", 4), "ab… [6 more bytes]");
    }
}
