use crate::app::SafeAppHandle;
use tokio::process::Command;
use crate::app::chord_runner::{ChordActionTask, ChordActionTaskRun, ChordActionTaskRunner};
use crate::models::{ChordAction, ChordShellAction};

pub struct ShellChordActionTaskRunner {
    _handle: SafeAppHandle,
}

impl ShellChordActionTaskRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { _handle: handle }
    }
}

impl ChordActionTaskRunner for ShellChordActionTaskRunner {
    fn start(&self, task: ChordActionTask) -> Result<Option<ChordActionTaskRun>> {
        let ChordAction::Shell(action) = task.action else {
            return Ok(None);
        };

        let join_handle = tauri::async_runtime::spawn(async move {
            run_shell_command(action.command.clone()).await
        });

        Ok(Some(ChordActionTaskRun {
            id: 0,
            task
        }))
    }

    fn end() {}

    fn abort(&self) {}
}

async fn run_shell_command(shell: String) {
    let mut command = Command::new("sh");
    command.arg("-c").arg(&shell);
    log::debug!("Running shell command: {:?}", command);

    match command.output().await {
        Ok(output) => log_shell_output(&shell, output),
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
