//! Parent-side driver for a `chord-native-host` process: spawning with an inherited socketpair
//! descriptor, the protocol handshake, generation loading, invocations, and crash detection.
//!
//! This lives in the protocol crate so Chord's supervisor, the `chord native-*` CLI commands and
//! the host's own integration tests share exactly one implementation.

use crate::{
    CACHE_DIR_ENV, HOST_FD_ENV, HandlerLoadError, HostRequest, HostResponse, InvocationContext,
    InvocationResult, NativeHandlerRegistration, PROTOCOL_VERSION, read_frame, write_frame,
};
use std::collections::VecDeque;
use std::os::fd::IntoRawFd;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};

/// The descriptor number the child receives the socket on.
const CHILD_SOCKET_FD: i32 = 3;

/// How much of the host's stderr is retained for crash diagnostics.
const STDERR_TAIL_CAPACITY: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostLogStream {
    Stdout,
    Stderr,
}

pub type LogSink = Arc<dyn Fn(HostLogStream, &str) + Send + Sync>;

pub struct HostSpawnOptions {
    /// Path to the `chord-native-host` executable.
    pub binary: PathBuf,
    /// Directory the host is allowed to load libraries from.
    pub cache_dir: PathBuf,
    /// Receives every stdout/stderr line the host prints.
    pub log_sink: Option<LogSink>,
    /// Maximum time to wait for the protocol handshake.
    pub ready_timeout: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("native host exited ({status}){}", format_tail(.stderr_tail))]
    Exited {
        status: String,
        stderr_tail: String,
    },
    #[error("native host disconnected{}", format_tail(.stderr_tail))]
    Disconnected { stderr_tail: String },
    #[error("native host did not respond within {0:?}")]
    TimedOut(Duration),
    #[error("native host protocol error: {0}")]
    Protocol(String),
    #[error("failed to spawn native host at {binary}: {source}")]
    Spawn {
        binary: PathBuf,
        source: std::io::Error,
    },
    #[error("native host io error: {0}")]
    Io(#[from] std::io::Error),
}

fn format_tail(tail: &str) -> String {
    let tail = tail.trim();
    if tail.is_empty() {
        String::new()
    } else {
        format!("\n--- native host stderr ---\n{tail}")
    }
}

#[derive(Debug)]
pub struct LoadOutcome {
    pub library_count: u32,
    pub handler_count: u32,
}

#[derive(Debug)]
pub struct InvokeOutcome {
    pub result: InvocationResult,
    /// Time the host spent inside the native call(s).
    pub native_duration: Duration,
    /// Round trip as measured by the parent, from before encoding to after decoding.
    pub round_trip: Duration,
}

pub struct HostProcess {
    child: Child,
    stream: UnixStream,
    pid: u32,
    stderr_tail: Arc<Mutex<VecDeque<u8>>>,
    generation_id: Option<u64>,
    next_invocation_id: u64,
}

impl HostProcess {
    pub async fn spawn(options: HostSpawnOptions) -> Result<Self, HostError> {
        let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
        let theirs_fd = theirs.into_raw_fd();

        let mut command = Command::new(&options.binary);
        command
            .env(HOST_FD_ENV, CHILD_SOCKET_FD.to_string())
            .env(CACHE_DIR_ENV, &options.cache_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // SAFETY: only async-signal-safe libc calls run between fork and exec.
        unsafe {
            command.pre_exec(move || {
                if theirs_fd == CHILD_SOCKET_FD {
                    // Clear FD_CLOEXEC so the descriptor survives exec.
                    if libc::fcntl(theirs_fd, libc::F_SETFD, 0) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                } else if libc::dup2(theirs_fd, CHILD_SOCKET_FD) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let spawn_result = command.spawn();
        // The parent never uses its copy of the child's end.
        // SAFETY: we own this descriptor (taken from `into_raw_fd`) and nothing else refers to it.
        unsafe {
            libc::close(theirs_fd);
        }
        let mut child = spawn_result.map_err(|source| HostError::Spawn {
            binary: options.binary.clone(),
            source,
        })?;
        let pid = child.id().unwrap_or_default();

        let stderr_tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL_CAPACITY)));
        if let Some(stdout) = child.stdout.take() {
            pump_lines(stdout, HostLogStream::Stdout, options.log_sink.clone(), None);
        }
        if let Some(stderr) = child.stderr.take() {
            pump_lines(
                stderr,
                HostLogStream::Stderr,
                options.log_sink.clone(),
                Some(Arc::clone(&stderr_tail)),
            );
        }

        ours.set_nonblocking(true)?;
        let stream = UnixStream::from_std(ours)?;
        let mut host = Self {
            child,
            stream,
            pid,
            stderr_tail,
            generation_id: None,
            next_invocation_id: 1,
        };

        match host
            .request(
                HostRequest::Hello {
                    protocol_version: PROTOCOL_VERSION,
                },
                options.ready_timeout,
            )
            .await?
        {
            HostResponse::Hello {
                protocol_version, ..
            } if protocol_version == PROTOCOL_VERSION => Ok(host),
            HostResponse::Hello {
                protocol_version, ..
            } => {
                host.kill().await;
                Err(HostError::Protocol(format!(
                    "host speaks protocol {protocol_version}, expected {PROTOCOL_VERSION}"
                )))
            }
            other => {
                host.kill().await;
                Err(HostError::Protocol(format!(
                    "unexpected handshake response: {other:?}"
                )))
            }
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn generation_id(&self) -> Option<u64> {
        self.generation_id
    }

    pub fn stderr_tail(&self) -> String {
        let tail = self.stderr_tail.lock().unwrap_or_else(|e| e.into_inner());
        String::from_utf8_lossy(&tail.iter().copied().collect::<Vec<_>>()).into_owned()
    }

    pub async fn load_generation(
        &mut self,
        generation_id: u64,
        handlers: Vec<NativeHandlerRegistration>,
        timeout: Duration,
    ) -> Result<Result<LoadOutcome, Vec<HandlerLoadError>>, HostError> {
        let response = self
            .request(
                HostRequest::LoadGeneration {
                    generation_id,
                    handlers,
                },
                timeout,
            )
            .await?;
        match response {
            HostResponse::GenerationLoaded {
                generation_id: loaded,
                library_count,
                handler_count,
            } if loaded == generation_id => {
                self.generation_id = Some(generation_id);
                Ok(Ok(LoadOutcome {
                    library_count,
                    handler_count,
                }))
            }
            HostResponse::GenerationLoadFailed {
                generation_id: failed,
                errors,
            } if failed == generation_id => Ok(Err(errors)),
            other => Err(HostError::Protocol(format!(
                "unexpected response to LoadGeneration: {other:?}"
            ))),
        }
    }

    pub async fn invoke(
        &mut self,
        handler_id: &str,
        event_arguments: Vec<String>,
        repeat: u32,
        context: InvocationContext,
        timeout: Duration,
    ) -> Result<InvokeOutcome, HostError> {
        let generation_id = self.generation_id.ok_or_else(|| {
            HostError::Protocol("no generation is loaded in the native host".into())
        })?;
        let invocation_id = self.next_invocation_id;
        self.next_invocation_id += 1;

        let started = Instant::now();
        let response = self
            .request(
                HostRequest::Invoke {
                    generation_id,
                    invocation_id,
                    handler_id: handler_id.to_string(),
                    event_arguments,
                    repeat,
                    context,
                },
                timeout,
            )
            .await?;
        let round_trip = started.elapsed();

        match response {
            HostResponse::InvocationFinished {
                invocation_id: finished,
                duration_ns,
                result,
            } if finished == invocation_id => Ok(InvokeOutcome {
                result,
                native_duration: Duration::from_nanos(duration_ns),
                round_trip,
            }),
            other => Err(HostError::Protocol(format!(
                "unexpected response to Invoke: {other:?}"
            ))),
        }
    }

    /// Asks the host to exit, waits up to `grace`, then kills it.
    pub async fn shutdown(mut self, grace: Duration) {
        let _ = tokio::time::timeout(grace, async {
            if write_frame(&mut self.stream, &HostRequest::Shutdown)
                .await
                .is_ok()
            {
                let _ = self.child.wait().await;
            }
        })
        .await;
        self.kill().await;
    }

    pub async fn kill(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }

    /// Sends one request and waits for its response, racing against host exit and the timeout.
    /// Any failure kills the host: after a crash, disconnect or timeout the process is unusable.
    async fn request(
        &mut self,
        request: HostRequest,
        timeout: Duration,
    ) -> Result<HostResponse, HostError> {
        let outcome = tokio::time::timeout(timeout, async {
            if let Err(error) = write_frame(&mut self.stream, &request).await {
                return Err(match self.child.try_wait() {
                    Ok(Some(status)) => exited(status, &self.stderr_tail),
                    _ => HostError::Protocol(format!("failed to send request: {error}")),
                });
            }
            tokio::select! {
                biased;
                response = read_frame::<_, HostResponse>(&mut self.stream) => match response {
                    Ok(HostResponse::ProtocolError { message }) => Err(HostError::Protocol(message)),
                    Ok(response) => Ok(response),
                    Err(crate::FrameError::Closed) => {
                        // Distinguish a crash from a plain disconnect when we can.
                        match tokio::time::timeout(Duration::from_millis(200), self.child.wait()).await {
                            Ok(Ok(status)) => Err(exited(status, &self.stderr_tail)),
                            _ => Err(HostError::Disconnected { stderr_tail: tail_string(&self.stderr_tail) }),
                        }
                    }
                    Err(error) => Err(HostError::Protocol(error.to_string())),
                },
                status = self.child.wait() => match status {
                    Ok(status) => Err(exited(status, &self.stderr_tail)),
                    Err(error) => Err(HostError::Io(error)),
                },
            }
        })
        .await;

        match outcome {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(error)) => {
                self.kill().await;
                Err(error)
            }
            Err(_elapsed) => {
                self.kill().await;
                Err(HostError::TimedOut(timeout))
            }
        }
    }
}

fn exited(status: ExitStatus, tail: &Arc<Mutex<VecDeque<u8>>>) -> HostError {
    HostError::Exited {
        status: describe_exit(status),
        stderr_tail: tail_string(tail),
    }
}

fn tail_string(tail: &Arc<Mutex<VecDeque<u8>>>) -> String {
    let tail = tail.lock().unwrap_or_else(|e| e.into_inner());
    String::from_utf8_lossy(&tail.iter().copied().collect::<Vec<_>>()).into_owned()
}

fn describe_exit(status: ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    if let Some(code) = status.code() {
        return format!("exit code {code}");
    }
    if let Some(signal) = status.signal() {
        let name = match signal {
            libc::SIGSEGV => "SIGSEGV",
            libc::SIGBUS => "SIGBUS",
            libc::SIGABRT => "SIGABRT",
            libc::SIGILL => "SIGILL",
            libc::SIGTRAP => "SIGTRAP",
            libc::SIGKILL => "SIGKILL",
            libc::SIGTERM => "SIGTERM",
            _ => "signal",
        };
        return format!("{name} ({signal})");
    }
    format!("{status}")
}

fn pump_lines<R>(
    reader: R,
    stream: HostLogStream,
    sink: Option<LogSink>,
    tail: Option<Arc<Mutex<VecDeque<u8>>>>,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(tail) = &tail {
                let mut tail = tail.lock().unwrap_or_else(|e| e.into_inner());
                for byte in line.bytes().chain(std::iter::once(b'\n')) {
                    if tail.len() == STDERR_TAIL_CAPACITY {
                        tail.pop_front();
                    }
                    tail.push_back(byte);
                }
            }
            if let Some(sink) = &sink {
                sink(stream, &line);
            }
        }
    });
}
