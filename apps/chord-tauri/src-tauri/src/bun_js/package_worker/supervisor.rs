//! Stable package/handler identities survive disposable worker processes.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex as AsyncMutex, Notify, Semaphore};
use tokio::time::Instant;

const HEALTHY_WINDOW: Duration = Duration::from_secs(60);

struct Registration {
    request: Request,
    remote_id: String,
}

struct State {
    worker: Arc<PackageWorker>,
    registrations: HashMap<String, Registration>,
    registration_order: Vec<String>,
    started: Instant,
    failed_worker: Option<String>,
    failures: u32,
    restart_at: Instant,
}

impl State {
    fn postpone_restart(&mut self) {
        self.failures = self.failures.saturating_add(1);
        let delay = Duration::from_millis(250 * (1_u64 << self.failures.min(8).saturating_sub(1)))
            .min(Duration::from_secs(30));
        self.restart_at = Instant::now() + delay;
    }

    fn observe_failure(&mut self) {
        if self.failed_worker.as_deref() == Some(&self.worker.id) {
            return;
        }
        if self.started.elapsed() >= HEALTHY_WINDOW {
            self.failures = 0;
        }
        self.failed_worker = Some(self.worker.id.clone());
        self.postpone_restart();
    }
}

pub(super) struct PackageSupervisor {
    pub(super) id: String,
    name: String,
    executable: PathBuf,
    config: WorkerConfig,
    handle: Option<AppHandle>,
    action_timeout: Duration,
    state: AsyncMutex<State>,
    // This separate pointer lets cancellation/shutdown interrupt a busy action
    // without waiting for its async state lock.
    active: Mutex<Arc<PackageWorker>>,
    slots: Semaphore,
    stopped: AtomicBool,
    stop_notice: Notify,
}

impl PackageSupervisor {
    pub(super) async fn spawn(
        executable: &Path,
        name: String,
        config: WorkerConfig,
        handle: Option<AppHandle>,
        action_timeout: Duration,
    ) -> Result<Arc<Self>> {
        let worker = PackageWorker::spawn(
            executable,
            name.clone(),
            config.clone(),
            handle.clone(),
            action_timeout,
        )
        .await?;
        let supervisor = Arc::new(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            executable: executable.into(),
            config,
            handle,
            action_timeout,
            active: Mutex::new(worker.clone()),
            state: AsyncMutex::new(State {
                worker,
                registrations: HashMap::new(),
                registration_order: Vec::new(),
                started: Instant::now(),
                failed_worker: None,
                failures: 0,
                restart_at: Instant::now(),
            }),
            slots: Semaphore::new(QUEUE_CAPACITY),
            stopped: AtomicBool::new(false),
            stop_notice: Notify::new(),
        });
        let weak = Arc::downgrade(&supervisor);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEALTH_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let Some(supervisor) = weak.upgrade() else {
                    break;
                };
                if supervisor.stopped.load(Ordering::Acquire) {
                    break;
                }
                // Recovery never queues behind a busy handler. The transport
                // actor already bounds that handler's execution time.
                if let Ok(mut state) = supervisor.state.try_lock() {
                    let _ = supervisor.ensure_ready(&mut state, false).await;
                }
            }
        });
        Ok(supervisor)
    }

    async fn ensure_ready(&self, state: &mut State, wait: bool) -> Result<()> {
        ensure!(
            !self.stopped.load(Ordering::Acquire),
            "package {} was unloaded",
            self.name
        );
        if state.worker.failure.lock().is_none() {
            return Ok(());
        }
        state.observe_failure();
        if Instant::now() < state.restart_at {
            if !wait {
                return Ok(());
            }
            tokio::select! {
                _ = tokio::time::sleep_until(state.restart_at) => {},
                _ = self.stop_notice.notified() => {},
            }
            ensure!(
                !self.stopped.load(Ordering::Acquire),
                "package {} was unloaded",
                self.name
            );
        }
        state.worker.stop();
        let restored = async {
            let worker = PackageWorker::spawn(
                &self.executable,
                self.name.clone(),
                self.config.clone(),
                self.handle.clone(),
                self.action_timeout,
            )
            .await?;
            // Publish before registration, so abort/shutdown can stop startup
            // factories too. Failed restoration never exposes partial mappings.
            *self.active.lock() = worker.clone();
            ensure!(
                !self.stopped.load(Ordering::Acquire),
                "package {} was unloaded",
                self.name
            );
            let mut ids = HashMap::new();
            for id in &state.registration_order {
                let registration = &state.registrations[id];
                let remote = worker.request(registration.request.clone()).await?;
                let remote = remote.as_str().context("invalid restored handler ID")?;
                ids.insert(id.clone(), remote.to_string());
            }
            ensure!(
                !self.stopped.load(Ordering::Acquire),
                "package {} was unloaded",
                self.name
            );
            Ok::<_, anyhow::Error>((worker, ids))
        }
        .await;
        match restored {
            Ok((worker, ids)) => {
                for (id, remote_id) in ids {
                    state.registrations.get_mut(&id).unwrap().remote_id = remote_id;
                }
                state.worker = worker;
                state.started = Instant::now();
                state.failed_worker = None;
                log::info!("package {} worker restarted; handlers restored", self.name);
                Ok(())
            }
            Err(error) => {
                self.active.lock().stop();
                state.postpone_restart();
                log::warn!(
                    "package {} worker recovery failed: {error:#}; retrying with backoff",
                    self.name
                );
                Err(error.context(format!(
                    "package {} worker recovery failed; will retry automatically",
                    self.name
                )))
            }
        }
    }

    pub(super) async fn request(&self, mut request: Request) -> Result<serde_json::Value> {
        let _slot = self
            .slots
            .try_acquire()
            .map_err(|_| anyhow::anyhow!("package {} request queue is full", self.name))?;
        let mut state = self.state.lock().await;
        ensure!(
            !self.stopped.load(Ordering::Acquire),
            "package {} was unloaded",
            self.name
        );
        if let Request::RetainHandlers(ids) = &request {
            let keep: HashSet<_> = ids.iter().map(String::as_str).collect();
            state
                .registrations
                .retain(|id, _| keep.contains(id.as_str()));
            state
                .registration_order
                .retain(|id| keep.contains(id.as_str()));
        }
        if matches!(
            request,
            Request::ReleaseSequences(_) | Request::RetainHandlers(_)
        ) && state.worker.failure.lock().is_some()
        {
            // A dead VM has no sequence values to release. Pruned registrations
            // are already reflected in the next background restoration.
            return Ok(serde_json::Value::Null);
        }
        self.ensure_ready(&mut state, true).await?;
        let registration = matches!(request, Request::Register { .. }).then(|| request.clone());
        match &mut request {
            Request::Invoke { action, .. } => {
                action.handler_id = state
                    .registrations
                    .get(&action.handler_id)
                    .context("package handler is no longer registered")?
                    .remote_id
                    .clone();
            }
            Request::RetainHandlers(ids) => {
                *ids = ids
                    .iter()
                    .filter_map(|id| {
                        state
                            .registrations
                            .get(id)
                            .map(|registration| registration.remote_id.clone())
                    })
                    .collect();
            }
            _ => {}
        }
        // Send once. A disconnected action may already have changed the app;
        // recovery restores factories but never replays the interrupted action.
        let result = state.worker.request(request).await;
        if state.worker.failure.lock().is_some() {
            state.observe_failure();
        }
        let value = result?;
        if let Some(request) = registration {
            let remote_id = value
                .as_str()
                .context("invalid handler ID from package worker")?
                .to_string();
            let id = uuid::Uuid::new_v4().to_string();
            state.registration_order.push(id.clone());
            state
                .registrations
                .insert(id.clone(), Registration { request, remote_id });
            return Ok(serde_json::Value::String(id));
        }
        Ok(value)
    }

    pub(super) fn abort(&self) {
        self.active.lock().stop();
    }

    pub(super) fn is_running(&self) -> bool {
        !self.stopped.load(Ordering::Acquire)
    }

    pub(super) fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.stop_notice.notify_waiters();
        self.abort();
    }
}

impl Drop for PackageSupervisor {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HandlerChordAction;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("chord-recovery-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(root.join("js")).unwrap();
            std::fs::write(root.join("package.json"), r#"{"name":"recovery-fixture"}"#).unwrap();
            Self(root)
        }

        async fn supervisor(&self) -> Arc<PackageSupervisor> {
            let executable = PathBuf::from(
                std::env::var_os("CHORD_PACKAGE_TEST_HOST")
                    .expect("set CHORD_PACKAGE_TEST_HOST to the freshly built chord executable"),
            );
            PackageSupervisor::spawn(
                &executable,
                "recovery-fixture".into(),
                WorkerConfig {
                    root: self.0.clone(),
                    cwd: self.0.clone(),
                },
                None,
                Duration::from_millis(500),
            )
            .await
            .unwrap()
        }

        async fn register(&self, supervisor: &PackageSupervisor, label: &str) -> String {
            let ledger = serde_json::to_string(&self.0.join("actions.json")).unwrap();
            let marker = serde_json::to_string(&self.0.join("crashed")).unwrap();
            let module = self.0.join("js/fixture.js");
            std::fs::write(
                &module,
                format!(
                    r#"
export default function(label) {{
    (globalThis.factoryOrder ??= []).push(label);
    return async function(mode) {{
        const ledger = Bun.file({ledger});
        const actions = await ledger.exists() ? await ledger.json() : [];
        actions.push(label + ":" + mode);
        await Bun.write(ledger, JSON.stringify(actions));
        if (mode === "abort-once" && !await Bun.file({marker}).exists()) {{
            await Bun.write({marker}, "crashed");
            process.abort();
        }}
        if (mode === "hang") await new Promise(() => {{}});
        if (mode === "throw") throw new Error("ordinary handler exception");
        if (mode === "order" && globalThis.factoryOrder.join(",") !== "first,second")
            throw new Error("factory order changed during recovery");
    }};
}}
"#
                ),
            )
            .unwrap();
            supervisor
                .request(Request::Register {
                    module: module.to_string_lossy().into_owned(),
                    raw: serde_json::json!({"name":"fixture"}),
                    pathslug: "fixture.toml".into(),
                    app_id: None,
                    args: vec![label.into()],
                })
                .await
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        }

        fn actions(&self) -> Vec<String> {
            serde_json::from_slice(&std::fs::read(self.0.join("actions.json")).unwrap()).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn invoke(id: &str, mode: &str) -> Request {
        Request::Invoke {
            action: HandlerChordAction {
                handler_id: id.into(),
                event_args: vec![mode.into()],
                input: "fixture".into(),
                sequence_id: None,
                assign_to: None,
                keyboard_event: None,
                focused_window_id: None,
            },
            app_id: None,
            count: 1,
        }
    }

    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn supervisor_restores_handlers_without_replaying_actions_or_blocking_other_packages() {
        let fixture = Fixture::new();
        let supervisor = fixture.supervisor().await;
        let first = fixture.register(&supervisor, "first").await;
        let second = fixture.register(&supervisor, "second").await;
        let stable_id = supervisor.id.clone();
        let original_pid = supervisor.active.lock().process.child.lock().id();
        let peer_fixture = Fixture::new();
        let peer = peer_fixture.supervisor().await;
        let peer_id = peer_fixture.register(&peer, "peer").await;
        let peer_pid = peer.active.lock().process.child.lock().id();
        let (failed, healthy) = tokio::join!(
            supervisor.request(invoke(&first, "abort-once")),
            tokio::time::timeout(
                Duration::from_millis(500),
                peer.request(invoke(&peer_id, "healthy"))
            ),
        );
        assert!(
            failed
                .unwrap_err()
                .to_string()
                .contains("exited or disconnected")
        );
        healthy
            .expect("another package's crash blocked a healthy worker")
            .unwrap();
        // No invocation triggers this restart: verify idle background recovery.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if supervisor.active.lock().process.child.lock().id() != original_pid {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the failed worker was not replaced automatically");
        supervisor.request(invoke(&second, "order")).await.unwrap();
        supervisor
            .request(invoke(&first, "after-recovery"))
            .await
            .unwrap();
        assert_eq!(supervisor.id, stable_id);
        assert_eq!(
            fixture.actions(),
            ["first:abort-once", "second:order", "first:after-recovery"]
        );
        assert_eq!(peer.active.lock().process.child.lock().id(), peer_pid);

        let recovered_pid = supervisor.active.lock().process.child.lock().id();
        assert!(
            supervisor
                .request(invoke(&first, "throw"))
                .await
                .unwrap_err()
                .to_string()
                .contains("ordinary handler exception")
        );
        assert_eq!(
            supervisor.active.lock().process.child.lock().id(),
            recovered_pid
        );
        supervisor
            .request(Request::RetainHandlers(vec![first.clone()]))
            .await
            .unwrap();
        supervisor.abort();
        supervisor
            .request(invoke(&first, "after-abort"))
            .await
            .unwrap();
        assert!(
            supervisor
                .request(invoke(&second, "removed"))
                .await
                .unwrap_err()
                .to_string()
                .contains("no longer registered")
        );
        let final_pid = supervisor.active.lock().process.child.lock().id();
        supervisor.stop();
        assert!(
            supervisor
                .request(invoke(&first, "unloaded"))
                .await
                .unwrap_err()
                .to_string()
                .contains("was unloaded")
        );
        tokio::time::sleep(HEALTH_INTERVAL + Duration::from_millis(100)).await;
        assert_eq!(
            supervisor.active.lock().process.child.lock().id(),
            final_pid
        );
        assert!(
            supervisor
                .active
                .lock()
                .process
                .child
                .lock()
                .try_wait()
                .unwrap()
                .is_some()
        );
        peer.stop();
    }

    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn supervisor_recovers_from_action_timeouts_and_keeps_queued_actions_once_only() {
        let fixture = Fixture::new();
        let supervisor = fixture.supervisor().await;
        let id = fixture.register(&supervisor, "first").await;
        let (hung, queued) = tokio::join!(
            supervisor.request(invoke(&id, "hang")),
            supervisor.request(invoke(&id, "queued")),
        );
        assert!(hung.unwrap_err().to_string().contains("timed out"));
        queued.unwrap();
        assert_eq!(fixture.actions(), ["first:hang", "first:queued"]);
        supervisor.stop();
    }
}
