use crate::app::SafeAppHandle;
use tokio::process::Command;
use crate::models::ShellChordAction;
use anyhow::Result;
use tauri::async_runtime::JoinHandle;

pub struct ShellChordActionTaskRunner {
    _handle: SafeAppHandle,
}

#[derive(Debug)]
pub struct ShellChordActionTaskRun {
    join_handle: JoinHandle<()>
}

impl ShellChordActionTaskRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { _handle: handle }
    }
}

impl ShellChordActionTaskRunner {
    pub fn start(&self, action: &ShellChordAction, num_times: u32) -> Result<ShellChordActionTaskRun> {
        let command = action.command.clone();
        let join_handle = tauri::async_runtime::spawn(async move {
            for _ in 0..num_times {
                run_shell_command(&command).await
            }
        });

        Ok(ShellChordActionTaskRun {
            join_handle
        })
    }

    pub async fn end(&self, task_run: ShellChordActionTaskRun) -> Result<()> {
        task_run.join_handle.await?;
        Ok(())
    }

    pub fn abort(&self, task_run: ShellChordActionTaskRun) -> Result<()> {
        task_run.join_handle.abort();
        Ok(())
    }
}

async fn run_shell_command(shell: &str) {
    let mut command = Command::new("sh");
    command.arg("-c").arg(shell);
    log::debug!("Running shell command: {:?}", command);

    match command.output().await {
        Ok(output) => log_shell_output(shell, output),
        Err(e) => {
            log::error!("failed to run shell command `{shell}`: {e}");
        }
    }
}

fn log_shell_output(shell: &str, output: std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
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
}
