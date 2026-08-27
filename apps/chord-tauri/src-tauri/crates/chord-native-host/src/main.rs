//! `chord-native-host`: a single persistent process that `dlopen`s every native handler library
//! referenced by the active Chord package generation and invokes them on request.
//!
//! Design constraints (see the native handlers plan):
//! - The request loop runs on the process main thread so AppKit/Accessibility calls in user code
//!   behave as expected. The tokio runtime is `current_thread` for the same reason.
//! - Libraries are loaded once per process and never unloaded; Chord starts a fresh host for
//!   every package generation, so process replacement is the unload boundary.
//! - Nothing here can make arbitrary native code safe. A crash in user code kills this process
//!   only; Chord supervises and restarts it.

#[cfg(unix)]
mod host {
    use anyhow::{Context, Result, anyhow};
    use chord_native_protocol::{
        CACHE_DIR_ENV, ENTRYPOINT_V1, ERROR_BUFFER_CAPACITY, FrameError, HOST_FD_ENV,
        HandlerLoadError, HostRequest, HostResponse, InvocationContext, InvocationResult,
        NativeHandlerRegistration, PROTOCOL_VERSION, abi_status, invocation_env, read_frame,
        write_frame,
    };
    use std::collections::HashMap;
    use std::ffi::{CString, c_char};
    use std::os::fd::FromRawFd;
    use std::path::{Path, PathBuf};
    use std::time::Instant;
    use tokio::net::UnixStream;

    type EntryFn = unsafe extern "C" fn(
        i32,
        *const *const c_char,
        i32,
        *const *const c_char,
        *mut u8,
        usize,
    ) -> i32;

    struct LoadedHandler {
        entry: EntryFn,
        handler_arguments: Vec<CString>,
        package_name: String,
        chords_file_pathslug: String,
    }

    struct HostState {
        cache_dir: PathBuf,
        generation_id: Option<u64>,
        libraries: HashMap<PathBuf, EntryFn>,
        handlers: HashMap<String, LoadedHandler>,
        error_buffer: Vec<u8>,
    }

    impl HostState {
        fn new(cache_dir: PathBuf) -> Self {
            Self {
                cache_dir,
                generation_id: None,
                libraries: HashMap::new(),
                handlers: HashMap::new(),
                error_buffer: vec![0u8; ERROR_BUFFER_CAPACITY],
            }
        }

        fn load_library(&mut self, path: &Path) -> Result<EntryFn> {
            let canonical = std::fs::canonicalize(path)
                .with_context(|| format!("library {} does not exist", path.display()))?;
            if !canonical.starts_with(&self.cache_dir) {
                anyhow::bail!(
                    "refusing to load {} because it is outside the native cache {}",
                    canonical.display(),
                    self.cache_dir.display()
                );
            }
            if let Some(entry) = self.libraries.get(&canonical) {
                return Ok(*entry);
            }

            // RTLD_NOW surfaces unresolved symbols at load time (not on first call);
            // RTLD_LOCAL keeps separately built handler modules from clobbering each other.
            let library = unsafe {
                libloading::os::unix::Library::open(
                    Some(&canonical),
                    libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
                )
            }
            .with_context(|| format!("failed to load {}", canonical.display()))?;

            let entry: EntryFn = unsafe {
                let symbol = library
                    .get::<EntryFn>(format!("{ENTRYPOINT_V1}\0").as_bytes())
                    .with_context(|| {
                        format!(
                            "{} does not export `{ENTRYPOINT_V1}` (ABI mismatch or not a Chord handler)",
                            canonical.display()
                        )
                    })?;
                *symbol
            };

            // Intentionally leaked: the function pointer must stay valid for the process lifetime.
            std::mem::forget(library);
            self.libraries.insert(canonical, entry);
            Ok(entry)
        }

        fn load_generation(
            &mut self,
            generation_id: u64,
            registrations: Vec<NativeHandlerRegistration>,
        ) -> HostResponse {
            let mut errors = Vec::new();
            let mut handlers = HashMap::new();

            for registration in registrations {
                let entry = match self.load_library(&registration.library_path) {
                    Ok(entry) => entry,
                    Err(error) => {
                        errors.push(HandlerLoadError {
                            handler_id: registration.handler_id,
                            library_path: registration.library_path,
                            message: format!("{error:#}"),
                        });
                        continue;
                    }
                };

                let handler_arguments = match to_c_strings(&registration.handler_arguments) {
                    Ok(args) => args,
                    Err(error) => {
                        errors.push(HandlerLoadError {
                            handler_id: registration.handler_id,
                            library_path: registration.library_path,
                            message: format!("invalid static handler argument: {error}"),
                        });
                        continue;
                    }
                };

                if handlers.contains_key(&registration.handler_id) {
                    errors.push(HandlerLoadError {
                        handler_id: registration.handler_id.clone(),
                        library_path: registration.library_path,
                        message: "duplicate handler id in generation".into(),
                    });
                    continue;
                }

                handlers.insert(
                    registration.handler_id,
                    LoadedHandler {
                        entry,
                        handler_arguments,
                        package_name: registration.package_name,
                        chords_file_pathslug: registration.chords_file_pathslug,
                    },
                );
            }

            if !errors.is_empty() {
                return HostResponse::GenerationLoadFailed {
                    generation_id,
                    errors,
                };
            }

            let handler_count = handlers.len() as u32;
            self.handlers = handlers;
            self.generation_id = Some(generation_id);
            HostResponse::GenerationLoaded {
                generation_id,
                library_count: self.libraries.len() as u32,
                handler_count,
            }
        }

        fn invoke(
            &mut self,
            generation_id: u64,
            invocation_id: u64,
            handler_id: &str,
            event_arguments: &[String],
            repeat: u32,
            context: &InvocationContext,
        ) -> HostResponse {
            let started = Instant::now();
            let finish = |result: InvocationResult| HostResponse::InvocationFinished {
                invocation_id,
                duration_ns: started.elapsed().as_nanos() as u64,
                result,
            };

            if self.generation_id != Some(generation_id) {
                return finish(InvocationResult::WrapperFailure {
                    message: format!(
                        "generation {generation_id} is not loaded (host has {:?})",
                        self.generation_id
                    ),
                });
            }
            let Some(handler) = self.handlers.get(handler_id) else {
                return finish(InvocationResult::WrapperFailure {
                    message: format!("handler {handler_id} is not registered in this generation"),
                });
            };
            let event_arguments = match to_c_strings(event_arguments) {
                Ok(args) => args,
                Err(error) => {
                    return finish(InvocationResult::InvalidArguments {
                        message: format!("invalid event argument: {error}"),
                    });
                }
            };

            let handler_argv: Vec<*const c_char> =
                handler.handler_arguments.iter().map(|s| s.as_ptr()).collect();
            let event_argv: Vec<*const c_char> =
                event_arguments.iter().map(|s| s.as_ptr()).collect();
            let (Ok(handler_argc), Ok(event_argc)) = (
                i32::try_from(handler_argv.len()),
                i32::try_from(event_argv.len()),
            ) else {
                return finish(InvocationResult::InvalidArguments {
                    message: "too many arguments".into(),
                });
            };

            let _env = InvocationEnvGuard::set(&[
                (invocation_env::PACKAGE_NAME, Some(handler.package_name.as_str())),
                (
                    invocation_env::CHORDS_FILE_PATHSLUG,
                    Some(handler.chords_file_pathslug.as_str()),
                ),
                (invocation_env::HANDLER_ID, Some(handler_id)),
                (
                    invocation_env::INVOCATION_ID,
                    Some(invocation_id.to_string().as_str()),
                ),
                (
                    invocation_env::FOCUSED_APP_ID,
                    context.focused_app_id.as_deref(),
                ),
            ]);

            let entry = handler.entry;
            let mut result = InvocationResult::Success;
            for _ in 0..repeat.max(1) {
                self.error_buffer[0] = 0;
                // SAFETY: argv pointers are backed by `CString`s that outlive the call; the error
                // buffer is a live, writable allocation of `ERROR_BUFFER_CAPACITY` bytes.
                let status = unsafe {
                    entry(
                        handler_argc,
                        handler_argv.as_ptr(),
                        event_argc,
                        event_argv.as_ptr(),
                        self.error_buffer.as_mut_ptr(),
                        self.error_buffer.len(),
                    )
                };
                if status == abi_status::SUCCESS {
                    continue;
                }
                let message = read_error_message(&self.error_buffer);
                result = match status {
                    abi_status::THROWN => InvocationResult::Thrown { message },
                    abi_status::INVALID_ARGUMENTS => InvocationResult::InvalidArguments { message },
                    other => InvocationResult::WrapperFailure {
                        message: format!("entrypoint returned status {other}: {message}"),
                    },
                };
                break;
            }

            // User code prints through C stdio, which is block-buffered on a pipe; flush every
            // stream so output reaches Chord's logs promptly instead of at host exit.
            // SAFETY: fflush(NULL) is always safe to call.
            unsafe {
                libc::fflush(std::ptr::null_mut());
            }

            finish(result)
        }
    }

    fn to_c_strings(values: &[String]) -> Result<Vec<CString>> {
        values
            .iter()
            .map(|value| {
                CString::new(value.as_str())
                    .map_err(|_| anyhow!("argument contains an embedded NUL byte"))
            })
            .collect()
    }

    fn read_error_message(buffer: &[u8]) -> String {
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        String::from_utf8_lossy(&buffer[..end]).into_owned()
    }

    /// Sets per-invocation environment variables and restores the previous values on drop.
    /// The host is single-threaded, which is what makes process-global env mutation acceptable.
    struct InvocationEnvGuard {
        previous: Vec<(&'static str, Option<String>)>,
    }

    impl InvocationEnvGuard {
        fn set(values: &[(&'static str, Option<&str>)]) -> Self {
            let mut previous = Vec::with_capacity(values.len());
            for (key, value) in values {
                previous.push((*key, std::env::var(key).ok()));
                // SAFETY: no other threads read or write the environment in this process.
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
            Self { previous }
        }
    }

    impl Drop for InvocationEnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.previous.drain(..) {
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    fn connect() -> Result<std::os::unix::net::UnixStream> {
        let fd: i32 = std::env::var(HOST_FD_ENV)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3);
        // SAFETY: the parent passed this descriptor explicitly for our exclusive use.
        let stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) };
        stream
            .set_nonblocking(true)
            .context("failed to set the IPC socket non-blocking")?;
        Ok(stream)
    }

    async fn serve(stream: UnixStream, cache_dir: PathBuf) -> Result<i32> {
        let mut stream = stream;
        let mut state = HostState::new(cache_dir);

        match read_frame::<_, HostRequest>(&mut stream).await? {
            HostRequest::Hello { protocol_version } if protocol_version == PROTOCOL_VERSION => {}
            HostRequest::Hello { protocol_version } => {
                write_frame(
                    &mut stream,
                    &HostResponse::ProtocolError {
                        message: format!(
                            "protocol version mismatch: chord speaks {protocol_version}, host speaks {PROTOCOL_VERSION}"
                        ),
                    },
                )
                .await?;
                return Ok(2);
            }
            other => anyhow::bail!("expected Hello as the first frame, got {other:?}"),
        }
        write_frame(
            &mut stream,
            &HostResponse::Hello {
                protocol_version: PROTOCOL_VERSION,
                pid: std::process::id(),
            },
        )
        .await?;

        loop {
            let request = match read_frame::<_, HostRequest>(&mut stream).await {
                Ok(request) => request,
                Err(FrameError::Closed) => return Ok(0),
                Err(error) => {
                    let _ = write_frame(
                        &mut stream,
                        &HostResponse::ProtocolError {
                            message: error.to_string(),
                        },
                    )
                    .await;
                    return Ok(2);
                }
            };

            let response = match request {
                HostRequest::Hello { .. } => HostResponse::ProtocolError {
                    message: "unexpected Hello after handshake".into(),
                },
                HostRequest::LoadGeneration {
                    generation_id,
                    handlers,
                } => state.load_generation(generation_id, handlers),
                HostRequest::Invoke {
                    generation_id,
                    invocation_id,
                    handler_id,
                    event_arguments,
                    repeat,
                    context,
                } => state.invoke(
                    generation_id,
                    invocation_id,
                    &handler_id,
                    &event_arguments,
                    repeat,
                    &context,
                ),
                HostRequest::Shutdown => return Ok(0),
            };

            write_frame(&mut stream, &response).await?;
        }
    }

    pub fn main() -> Result<i32> {
        let cache_dir = std::env::var_os(CACHE_DIR_ENV)
            .map(PathBuf::from)
            .with_context(|| format!("{CACHE_DIR_ENV} must be set"))?;
        let cache_dir = std::fs::canonicalize(&cache_dir)
            .with_context(|| format!("{CACHE_DIR_ENV}={} does not exist", cache_dir.display()))?;
        let std_stream = connect()?;

        // current_thread + block_on keeps every native invocation on the process main thread.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to build host runtime")?;
        runtime.block_on(async move {
            let stream = UnixStream::from_std(std_stream)
                .context("failed to register the IPC socket with tokio")?;
            serve(stream, cache_dir).await
        })
    }
}

fn main() {
    #[cfg(unix)]
    {
        match host::main() {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("chord-native-host: {error:#}");
                std::process::exit(2);
            }
        }
    }
    #[cfg(not(unix))]
    {
        eprintln!("chord-native-host is not supported on this platform yet");
        std::process::exit(2);
    }
}
