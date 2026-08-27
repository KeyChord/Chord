use crate::app::native_host::NativeInvocationError;
use crate::constants::{
    NATIVE_CRASH_LOOP_LIMIT, NATIVE_CRASH_LOOP_WINDOW, NATIVE_HOST_READY_TIMEOUT,
    NATIVE_HOST_SHUTDOWN_GRACE, NATIVE_INVOCATION_TIMEOUT,
};
use anyhow::{Context, Result};
use chord_native_protocol::client::{
    HostError, HostLogStream, HostProcess, HostSpawnOptions, LogSink,
};
use chord_native_protocol::{InvocationContext, InvocationResult, NativeHandlerRegistration};
use nject::injectable;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::{AppHandle, Manager};

/// Environment override for the host executable (handy for `cargo run`-style development).
pub const NATIVE_HOST_BIN_ENV: &str = "CHORD_NATIVE_HOST_BIN";

/// Name of the sidecar executable placed next to the main Chord binary by Tauri.
pub const NATIVE_HOST_BIN_NAME: &str = "chord-native-host";

/// Owns the single `chord-native-host` process, the desired handler generation, and all
/// crash/timeout recovery. Invocations are serialized by design (arbitrary native code and most
/// desktop APIs are not thread-safe), so the async mutex around the state doubles as the queue.
#[injectable]
pub struct NativeHostSupervisor {
    handle: AppHandle,

    #[inject(tokio::sync::Mutex::new(HostState::default()))]
    state: tokio::sync::Mutex<HostState>,

    /// PID of the live host, readable without the async lock so `abort_active` can kill a host
    /// that is currently blocked inside user code.
    #[inject(parking_lot::Mutex::new(None))]
    live_pid: parking_lot::Mutex<Option<u32>>,

    #[inject(AtomicBool::new(false))]
    abort_requested: AtomicBool,
}

#[derive(Default)]
struct HostState {
    host: Option<HostProcess>,
    generation_id: u64,
    /// Registrations that should be loaded in the host; reloaded verbatim after a restart.
    desired: Arc<Vec<NativeHandlerRegistration>>,
    /// Handlers that must not be invoked, with the reason (load failure or crash loop).
    disabled: HashMap<String, String>,
    crashes: HashMap<String, VecDeque<Instant>>,
}

pub struct GenerationSummary {
    pub generation_id: u64,
    pub handler_count: usize,
    pub library_count: u32,
    pub failed: Vec<(String, String)>,
}

impl NativeHostSupervisor {
    /// Directory under which native libraries are materialized; the host may only load from here.
    pub fn cache_dir(&self) -> Result<PathBuf> {
        let dir = self
            .handle
            .path()
            .app_cache_dir()
            .context("failed to resolve app cache dir")?
            .join("native-packages");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn host_binary_path() -> Result<PathBuf> {
        if let Some(path) = std::env::var_os(NATIVE_HOST_BIN_ENV) {
            return Ok(PathBuf::from(path));
        }
        let exe = std::env::current_exe().context("failed to resolve current executable")?;
        let dir = exe.parent().context("executable has no parent directory")?;
        let candidate = dir.join(NATIVE_HOST_BIN_NAME);
        anyhow::ensure!(
            candidate.exists(),
            "{} not found next to {} (set {} to override)",
            NATIVE_HOST_BIN_NAME,
            exe.display(),
            NATIVE_HOST_BIN_ENV
        );
        Ok(candidate)
    }

    pub fn log_sink() -> LogSink {
        Arc::new(|stream, line| match stream {
            HostLogStream::Stdout => log::info!(target: "native-host", "{line}"),
            HostLogStream::Stderr => log::warn!(target: "native-host", "{line}"),
        })
    }

    /// Replaces the active generation. A fresh host is started and fully loaded *before* the old
    /// one is retired, so there is never a window where a registered handler cannot be invoked.
    /// Handlers whose library fails to load are disabled individually; the rest still activate.
    pub async fn activate_generation(
        &self,
        registrations: Vec<NativeHandlerRegistration>,
    ) -> Result<GenerationSummary> {
        let mut state = self.state.lock().await;
        state.generation_id += 1;
        let generation_id = state.generation_id;
        state.disabled.clear();
        state.crashes.clear();

        let old_host = state.host.take();
        *self.live_pid.lock() = None;

        if registrations.is_empty() {
            state.desired = Arc::new(Vec::new());
            drop(state);
            if let Some(old) = old_host {
                log::info!("no native handlers registered; stopping native host {}", old.pid());
                old.shutdown(NATIVE_HOST_SHUTDOWN_GRACE).await;
            }
            return Ok(GenerationSummary {
                generation_id,
                handler_count: 0,
                library_count: 0,
                failed: Vec::new(),
            });
        }

        let (host, loaded, failed) = self.spawn_and_load(generation_id, registrations).await?;
        let summary = GenerationSummary {
            generation_id,
            handler_count: loaded.len(),
            library_count: host.as_ref().map(|(_, count)| *count).unwrap_or(0),
            failed: failed.clone(),
        };
        for (handler_id, reason) in failed {
            log::error!("native handler {handler_id} failed to load: {reason}");
            state.disabled.insert(handler_id, reason);
        }
        state.desired = Arc::new(loaded);
        if let Some((host, _)) = host {
            log::info!(
                "native host {} ready with generation {} ({} handlers)",
                host.pid(),
                generation_id,
                summary.handler_count
            );
            *self.live_pid.lock() = Some(host.pid());
            state.host = Some(host);
        }
        drop(state);

        if let Some(old) = old_host {
            old.shutdown(NATIVE_HOST_SHUTDOWN_GRACE).await;
        }
        Ok(summary)
    }

    /// Runs one handler. Errors describe exactly what happened and whether the host restarted.
    pub async fn invoke(
        &self,
        handler_id: &str,
        event_arguments: Vec<String>,
        repeat: u32,
        context: InvocationContext,
    ) -> Result<(), NativeInvocationError> {
        let mut state = self.state.lock().await;

        if let Some(reason) = state.disabled.get(handler_id) {
            return Err(NativeInvocationError::HandlerDisabled {
                handler_id: handler_id.to_string(),
                reason: reason.clone(),
            });
        }
        if !state.desired.iter().any(|r| r.handler_id == handler_id) {
            return Err(NativeInvocationError::HostUnavailable(format!(
                "handler {handler_id} is not part of the active native generation"
            )));
        }

        if state.host.is_none() {
            // A previous restart failed; try again now rather than staying dead forever.
            self.restart_locked(&mut state).await;
        }
        let Some(host) = state.host.as_mut() else {
            return Err(NativeInvocationError::HostUnavailable(
                "the native host could not be started; see earlier log entries".into(),
            ));
        };

        self.abort_requested.store(false, Ordering::SeqCst);
        let outcome = host
            .invoke(
                handler_id,
                event_arguments,
                repeat,
                context,
                NATIVE_INVOCATION_TIMEOUT,
            )
            .await;

        match outcome {
            Ok(outcome) => {
                log::debug!(
                    "native handler {handler_id} finished in {:?} (round trip {:?})",
                    outcome.native_duration,
                    outcome.round_trip
                );
                match outcome.result {
                    InvocationResult::Success => Ok(()),
                    InvocationResult::Thrown { message } => {
                        Err(NativeInvocationError::Thrown { message })
                    }
                    InvocationResult::InvalidArguments { message } => {
                        Err(NativeInvocationError::InvalidArguments { message })
                    }
                    InvocationResult::WrapperFailure { message } => {
                        Err(NativeInvocationError::WrapperFailure { message })
                    }
                }
            }
            Err(error) => {
                // The client already killed the host; it is unusable from here on.
                state.host = None;
                *self.live_pid.lock() = None;
                let aborted = self.abort_requested.swap(false, Ordering::SeqCst);

                let error = match error {
                    HostError::Exited { .. } | HostError::Disconnected { .. } if aborted => {
                        NativeInvocationError::Aborted {
                            handler_id: handler_id.to_string(),
                        }
                    }
                    HostError::Exited { .. } | HostError::Disconnected { .. } => {
                        self.record_crash(&mut state, handler_id);
                        NativeInvocationError::HostCrashed {
                            handler_id: handler_id.to_string(),
                            source: error,
                        }
                    }
                    HostError::TimedOut(timeout) => {
                        self.record_crash(&mut state, handler_id);
                        NativeInvocationError::TimedOut {
                            handler_id: handler_id.to_string(),
                            timeout,
                        }
                    }
                    other => NativeInvocationError::Protocol(other.to_string()),
                };

                self.restart_locked(&mut state).await;
                Err(error)
            }
        }
    }

    /// Kills the host even while an invocation is blocked inside native code. The blocked
    /// invocation completes with `Aborted`, and the host is restarted by that code path.
    pub fn abort_active(&self) {
        let Some(pid) = *self.live_pid.lock() else {
            return;
        };
        self.abort_requested.store(true, Ordering::SeqCst);
        log::warn!("aborting native host {pid}");
        #[cfg(unix)]
        // SAFETY: sending a signal to a PID we spawned and still track.
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }

    pub async fn shutdown(&self) {
        let mut state = self.state.lock().await;
        *self.live_pid.lock() = None;
        if let Some(host) = state.host.take() {
            host.shutdown(NATIVE_HOST_SHUTDOWN_GRACE).await;
        }
    }

    pub async fn host_pid(&self) -> Option<u32> {
        *self.live_pid.lock()
    }

    fn record_crash(&self, state: &mut HostState, handler_id: &str) {
        let now = Instant::now();
        let history = state.crashes.entry(handler_id.to_string()).or_default();
        history.push_back(now);
        while history
            .front()
            .is_some_and(|t| now.duration_since(*t) > NATIVE_CRASH_LOOP_WINDOW)
        {
            history.pop_front();
        }
        if history.len() >= NATIVE_CRASH_LOOP_LIMIT {
            let reason = format!(
                "crashed or hung {} times within {:?}; reload packages or rebuild the handler to re-enable it",
                history.len(),
                NATIVE_CRASH_LOOP_WINDOW
            );
            log::error!("disabling native handler {handler_id}: {reason}");
            state.disabled.insert(handler_id.to_string(), reason);
        }
    }

    /// Spawns a replacement host for the current desired generation. Failures are logged and
    /// leave `state.host` empty; the next invocation retries.
    async fn restart_locked(&self, state: &mut HostState) {
        let desired: Vec<_> = state
            .desired
            .iter()
            .filter(|r| !state.disabled.contains_key(&r.handler_id))
            .cloned()
            .collect();
        if desired.is_empty() {
            return;
        }
        match self.spawn_and_load(state.generation_id, desired).await {
            Ok((Some((host, _)), _, failed)) => {
                for (handler_id, reason) in failed {
                    log::error!("native handler {handler_id} failed to load after restart: {reason}");
                    state.disabled.insert(handler_id, reason);
                }
                log::info!("native host restarted as pid {}", host.pid());
                *self.live_pid.lock() = Some(host.pid());
                state.host = Some(host);
            }
            Ok((None, _, failed)) => {
                for (handler_id, reason) in failed {
                    state.disabled.insert(handler_id, reason);
                }
            }
            Err(error) => log::error!("failed to restart native host: {error:#}"),
        }
    }

    /// Starts a host and loads `registrations`, dropping (and reporting) any handler that fails
    /// to load so the remaining ones still work. Returns the host with its loaded library count,
    /// the registrations that loaded, and `(handler_id, reason)` for those that did not.
    #[allow(clippy::type_complexity)]
    async fn spawn_and_load(
        &self,
        generation_id: u64,
        registrations: Vec<NativeHandlerRegistration>,
    ) -> Result<(
        Option<(HostProcess, u32)>,
        Vec<NativeHandlerRegistration>,
        Vec<(String, String)>,
    )> {
        let binary = Self::host_binary_path()?;
        let cache_dir = self.cache_dir()?;
        let mut host = HostProcess::spawn(HostSpawnOptions {
            binary,
            cache_dir,
            log_sink: Some(Self::log_sink()),
            ready_timeout: NATIVE_HOST_READY_TIMEOUT,
        })
        .await
        .context("failed to start the native host")?;
        log::debug!("spawned native host pid {}", host.pid());

        let mut remaining = registrations;
        let mut failed = Vec::new();
        loop {
            match host
                .load_generation(generation_id, remaining.clone(), NATIVE_HOST_READY_TIMEOUT)
                .await
                .context("native host failed while loading the generation")?
            {
                Ok(outcome) => {
                    return Ok((Some((host, outcome.library_count)), remaining, failed));
                }
                Err(errors) => {
                    let failed_ids: Vec<_> = errors.iter().map(|e| e.handler_id.clone()).collect();
                    for error in errors {
                        failed.push((error.handler_id, error.message));
                    }
                    remaining.retain(|r| !failed_ids.contains(&r.handler_id));
                    if remaining.is_empty() {
                        host.shutdown(NATIVE_HOST_SHUTDOWN_GRACE).await;
                        return Ok((None, remaining, failed));
                    }
                }
            }
        }
    }
}
