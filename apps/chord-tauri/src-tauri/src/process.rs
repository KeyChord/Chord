use anyhow::{Context, Result, bail};
use std::fmt::Write;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

pub const COMMAND_OUTPUT_LIMIT: usize = 256 * 1024;
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(5 * 60);

const READ_BUFFER_SIZE: usize = 16 * 1024;

#[derive(Debug)]
pub struct BoundedOutput {
    pub status: ExitStatus,
    pub stderr: BoundedStream,
    pub stdout: BoundedStream,
}

#[derive(Debug)]
pub struct BoundedStream {
    bytes: Vec<u8>,
    total_bytes: u64,
}

impl BoundedStream {
    pub fn is_truncated(&self) -> bool {
        self.total_bytes > self.bytes.len() as u64
    }

    pub fn to_lossy_text(&self) -> String {
        let mut text = String::from_utf8_lossy(&self.bytes).into_owned();
        if self.is_truncated() {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            let _ = write!(
                text,
                "[output truncated: retained {} of {} bytes]",
                self.bytes.len(),
                self.total_bytes
            );
        }
        text
    }
}

/// Runs a child while continuously draining stdout and stderr, retaining at most `limit` bytes
/// from each stream. `kill_on_drop` and a separate Unix process group ensure aborting the future
/// cannot leave a verbose child running in the background.
pub async fn bounded_output(
    command: &mut Command,
    limit: usize,
    timeout: Duration,
) -> Result<BoundedOutput> {
    command
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    command.process_group(0);

    let mut child = command.spawn().context("failed to spawn child process")?;
    let child_id = child.id();
    let mut process_group = ChildProcessGroupGuard::new(child_id);
    let stdout = child.stdout.take().context("child stdout was not piped")?;
    let stderr = child.stderr.take().context("child stderr was not piped")?;
    let mut stdout_task = tokio::spawn(read_bounded(stdout, limit));
    let mut stderr_task = tokio::spawn(read_bounded(stderr, limit));

    let capture = async {
        let status = child
            .wait()
            .await
            .context("failed to wait for child process")?;
        let stdout = (&mut stdout_task)
            .await
            .context("stdout reader task failed")?
            .context("failed to read child stdout")?;
        let stderr = (&mut stderr_task)
            .await
            .context("stderr reader task failed")?
            .context("failed to read child stderr")?;
        Ok::<_, anyhow::Error>((status, stdout, stderr))
    };

    let (status, stdout, stderr) = match tokio::time::timeout(timeout, capture).await {
        Ok(output) => output?,
        Err(_) => {
            process_group.kill();
            let _ = child.kill().await;
            let _ = child.wait().await;
            stdout_task.abort();
            stderr_task.abort();
            bail!("command timed out after {} seconds", timeout.as_secs());
        }
    };
    process_group.disarm();

    Ok(BoundedOutput {
        status,
        stderr,
        stdout,
    })
}

async fn read_bounded<R>(mut reader: R, limit: usize) -> std::io::Result<BoundedStream>
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::with_capacity(limit.min(READ_BUFFER_SIZE));
    let mut total_bytes = 0_u64;
    let mut buffer = [0_u8; READ_BUFFER_SIZE];

    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        total_bytes = total_bytes.saturating_add(read as u64);
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
    }

    Ok(BoundedStream { bytes, total_bytes })
}

struct ChildProcessGroupGuard {
    armed: bool,
    child_id: Option<u32>,
}

impl ChildProcessGroupGuard {
    fn new(child_id: Option<u32>) -> Self {
        Self {
            armed: true,
            child_id,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn kill(&self) {
        #[cfg(unix)]
        {
            let Some(child_id) = self.child_id.and_then(|id| i32::try_from(id).ok()) else {
                return;
            };
            // The command is placed in a fresh process group above, so a negative pid is scoped
            // to this child and any descendants it spawned.
            unsafe {
                libc::kill(-child_id, libc::SIGKILL);
            }
        }
    }
}

impl Drop for ChildProcessGroupGuard {
    fn drop(&mut self) {
        if self.armed {
            self.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounds_both_output_streams_while_draining_the_child() {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("i=0; while [ $i -lt 4096 ]; do printf x; printf y >&2; i=$((i + 1)); done");

        let output = bounded_output(&mut command, 128, Duration::from_secs(5))
            .await
            .unwrap();

        assert!(output.status.success());
        assert!(output.stdout.is_truncated());
        assert!(output.stderr.is_truncated());
        assert!(output.stdout.to_lossy_text().len() < 256);
        assert!(output.stderr.to_lossy_text().len() < 256);
    }

    #[tokio::test]
    async fn times_out_a_long_running_command() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("sleep 30");

        let error = bounded_output(&mut command, 128, Duration::from_millis(50))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn times_out_when_a_background_descendant_keeps_the_pipes_open() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("sleep 30 &");

        let error = bounded_output(&mut command, 128, Duration::from_millis(50))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("timed out"));
    }
}
