use crate::app::chord_runner::ChordActionTask;
use crate::models::ShellChordAction;
use crate::process::{COMMAND_OUTPUT_LIMIT, COMMAND_TIMEOUT, bounded_output};
use anyhow::{Context, Result, bail};
use nject::injectable;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;
use tokio::process::Command;
use tokio::sync::Semaphore;

const MAX_CONCURRENT_SHELL_COMMANDS: usize = 16;
static SHELL_COMMAND_SLOTS: Semaphore = Semaphore::const_new(MAX_CONCURRENT_SHELL_COMMANDS);

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

    pub async fn end(&self, task_run: ShellChordActionTaskRun) -> Result<()> {
        task_run.join_handle.await??;
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
    let mut command = Command::new("sh");
    command.arg("-c").arg(shell);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    log::debug!("Running shell command: {:?}", command);

    let output = bounded_output(&mut command, COMMAND_OUTPUT_LIMIT, COMMAND_TIMEOUT)
        .await
        .with_context(|| format!("failed to run shell command `{shell}`"))?;
    log_shell_output(shell, output)
}

fn log_shell_output(shell: &str, output: crate::process::BoundedOutput) -> Result<()> {
    let stdout = output.stdout.to_lossy_text();
    let stderr = output.stderr.to_lossy_text();
    let stdout = stdout.trim();
    let stderr = stderr.trim();
    let exit_code = output.status.code();

    if output.status.success() {
        log::debug!(
            "shell command succeeded with exit code {:?}: {}",
            exit_code,
            shell
        );
    } else {
        log::error!(
            "shell command failed with exit code {:?}: {}",
            exit_code,
            shell
        );
    }

    if !stdout.is_empty() {
        log::debug!("shell stdout: {stdout}");
    }

    if !stderr.is_empty() {
        log::debug!("shell stderr: {stderr}");
    }

    if !output.status.success() {
        bail!("shell command `{shell}` failed with exit code {exit_code:?}");
    }

    Ok(())
}
