//! Stable package/handler identities survive disposable worker processes.
//!
//! A worker also exits once its package has gone unused for a while, unless the package
//! has work of its own between requests. The next request, or Chord Mode starting
//! (`warm`), starts a new one and restores its handlers, as after a crash.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex as AsyncMutex, Notify, Semaphore};
use tokio::time::Instant;

const HEALTHY_WINDOW: Duration = Duration::from_secs(60);

/// How long a worker may go unused before it exits. Each is a whole Bun process, about
/// 13 MB. A pause between chords keeps it; starting it again takes 0.2–0.6 s, which
/// Chord Mode hides by starting it while the chord is typed.
pub(super) const IDLE_EXIT: Duration = Duration::from_secs(120);

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
    /// The worker exited because it went unused, not because it failed: it starts again
    /// on the next request, without backoff, and the failure restarter leaves it.
    retired: bool,
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
    /// Told by a worker that failed or was stopped, and when the package is unloaded.
    worker_ended: Arc<Notify>,
    idle_exit: Duration,
    last_used: Mutex<Instant>,
    /// Told when the package is used, so the idle exit counts from then, and on unload.
    used: Arc<Notify>,
}

impl PackageSupervisor {
    pub(super) async fn spawn(
        executable: &Path,
        name: String,
        config: WorkerConfig,
        handle: Option<AppHandle>,
        action_timeout: Duration,
        idle_exit: Duration,
    ) -> Result<Arc<Self>> {
        let worker_ended = Arc::new(Notify::new());
        let worker = PackageWorker::spawn(
            executable,
            name.clone(),
            config.clone(),
            handle.clone(),
            action_timeout,
            worker_ended.clone(),
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
                retired: false,
            }),
            slots: Semaphore::new(QUEUE_CAPACITY),
            stopped: AtomicBool::new(false),
            stop_notice: Notify::new(),
            worker_ended: worker_ended.clone(),
            idle_exit,
            last_used: Mutex::new(Instant::now()),
            used: Arc::new(Notify::new()),
        });
        let weak = Arc::downgrade(&supervisor);
        tokio::spawn(async move {
            // Replaces a failed worker as soon as it fails, so the next chord finds a
            // ready one. Each failed attempt waits out its backoff before the next.
            loop {
                worker_ended.notified().await;
                loop {
                    let Some(supervisor) = weak.upgrade() else {
                        return;
                    };
                    if supervisor.stopped.load(Ordering::Acquire) {
                        return;
                    }
                    let mut state = supervisor.state.lock().await;
                    if state.retired || supervisor.ensure_ready(&mut state).await.is_ok() {
                        break;
                    }
                }
            }
        });
        let weak = Arc::downgrade(&supervisor);
        let used = supervisor.used.clone();
        tokio::spawn(async move {
            // Waits on a timer to the idle deadline, which each use moves, and on the next
            // use while the worker is gone or busy with work of its own. Nothing polls it.
            loop {
                let due = {
                    let Some(supervisor) = weak.upgrade() else {
                        return;
                    };
                    if supervisor.stopped.load(Ordering::Acquire) {
                        return;
                    }
                    *supervisor.last_used.lock() + supervisor.idle_exit
                };
                tokio::select! {
                    _ = tokio::time::sleep_until(due) => {}
                    _ = used.notified() => continue,
                }
                let Some(supervisor) = weak.upgrade() else {
                    return;
                };
                let next = supervisor.retire_if_unused().await;
                drop(supervisor);
                if next == IdleCheck::WaitForUse {
                    used.notified().await;
                }
            }
        });
        Ok(supervisor)
    }

    /// Notes a use of the package, from which the idle exit counts again.
    fn touch(&self) {
        *self.last_used.lock() = Instant::now();
        self.used.notify_one();
    }

    /// Stops the worker if the package has gone unused for `idle_exit` and has no work
    /// of its own.
    async fn retire_if_unused(&self) -> IdleCheck {
        // A request holds the state for as long as it runs, and touches when it ends.
        let Ok(mut state) = self.state.try_lock() else {
            return IdleCheck::WaitForUse;
        };
        if self.stopped.load(Ordering::Acquire)
            || state.retired
            || state.worker.failure.lock().is_some()
        {
            // Gone already, or failed and left to the restarter, which touches.
            return IdleCheck::WaitForUse;
        }
        if self.last_used.lock().elapsed() < self.idle_exit {
            return IdleCheck::Recheck;
        }
        // Asked of the worker itself, not through `request`, which would count as a use.
        let own_work = state.worker.request(Request::HasOwnWork).await;
        if !matches!(own_work, Ok(serde_json::Value::Bool(false))) {
            // It has, or can't say. The next use moves the deadline and asks again.
            return IdleCheck::WaitForUse;
        }
        log::info!(
            "package {} worker exited after {} s unused; it starts again when needed",
            self.name,
            self.idle_exit.as_secs()
        );
        state.retired = true;
        state.worker.stop();
        IdleCheck::WaitForUse
    }

    /// Starts the worker again if it exited while unused, and counts this as a use.
    pub(super) async fn warm(&self) {
        {
            let mut state = self.state.lock().await;
            if state.retired
                && let Err(error) = self.ensure_ready(&mut state).await
            {
                log::warn!("package {} worker did not start: {error:#}", self.name);
            }
        }
        self.touch();
    }

    async fn ensure_ready(&self, state: &mut State) -> Result<()> {
        ensure!(
            !self.stopped.load(Ordering::Acquire),
            "package {} was unloaded",
            self.name
        );
        if state.worker.failure.lock().is_none() {
            return Ok(());
        }
        // A worker that exited while unused starts again at once.
        let retired = std::mem::take(&mut state.retired);
        if !retired {
            state.observe_failure();
        }
        if !retired && Instant::now() < state.restart_at {
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
                self.worker_ended.clone(),
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
                if retired {
                    log::info!("package {} worker started again", self.name);
                } else {
                    log::info!("package {} worker restarted; handlers restored", self.name);
                }
                // The idle exit counts from the new worker, also when no request started it.
                self.touch();
                Ok(())
            }
            Err(error) => {
                // From here on it is a failure, which the restarter retries with backoff.
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

    pub(super) async fn request(&self, request: Request) -> Result<serde_json::Value> {
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
        if matches!(request, Request::Lifecycle { .. }) && state.retired {
            // Only a package without lifecycle callbacks is let go while unused.
            return Ok(serde_json::Value::Null);
        }
        let result = self.request_ready(&mut state, request).await;
        self.touch();
        result
    }

    async fn request_ready(
        &self,
        state: &mut State,
        mut request: Request,
    ) -> Result<serde_json::Value> {
        self.ensure_ready(state).await?;
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
        self.worker_ended.notify_one();
        self.used.notify_one();
        self.abort();
    }
}

#[derive(PartialEq)]
enum IdleCheck {
    /// The deadline moved; wait for the new one.
    Recheck,
    /// Nothing to do until the package is used again.
    WaitForUse,
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
            self.spawn_supervisor(Duration::from_millis(500), IDLE_EXIT)
                .await
        }

        async fn supervisor_exiting_after(&self, idle_exit: Duration) -> Arc<PackageSupervisor> {
            self.spawn_supervisor(Duration::from_secs(10), idle_exit)
                .await
        }

        async fn spawn_supervisor(
            &self,
            action_timeout: Duration,
            idle_exit: Duration,
        ) -> Arc<PackageSupervisor> {
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
                action_timeout,
                idle_exit,
            )
            .await
            .unwrap()
        }

        /// Registers the default export of a module with this source.
        async fn register_source(
            &self,
            supervisor: &PackageSupervisor,
            name: &str,
            source: &str,
        ) -> String {
            let module = self.0.join("js").join(format!("{name}.js"));
            std::fs::write(&module, source).unwrap();
            supervisor
                .request(Request::Register {
                    module: module.to_string_lossy().into_owned(),
                    raw: serde_json::json!({"name":"fixture"}),
                    pathslug: "fixture.toml".into(),
                    app_id: None,
                    args: vec![],
                })
                .await
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
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
        tokio::time::sleep(Duration::from_millis(1_100)).await;
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

    fn pid(supervisor: &PackageSupervisor) -> u32 {
        supervisor.active.lock().process.child.lock().id()
    }

    fn has_exited(supervisor: &PackageSupervisor) -> bool {
        supervisor
            .active
            .lock()
            .process
            .child
            .lock()
            .try_wait()
            .unwrap()
            .is_some()
    }

    /// Waits for the worker to exit on its own, or fails after `limit`.
    async fn exited_within(supervisor: &PackageSupervisor, limit: Duration) -> bool {
        tokio::time::timeout(limit, async {
            while !has_exited(supervisor) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .is_ok()
    }

    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn an_unused_worker_exits_and_starts_again_with_its_handlers() {
        let fixture = Fixture::new();
        let supervisor = fixture
            .supervisor_exiting_after(Duration::from_millis(300))
            .await;
        let first = fixture.register(&supervisor, "first").await;
        let second = fixture.register(&supervisor, "second").await;
        supervisor.request(invoke(&first, "used")).await.unwrap();
        let original = pid(&supervisor);
        assert!(
            exited_within(&supervisor, Duration::from_secs(5)).await,
            "an unused worker kept running"
        );
        assert!(supervisor.state.lock().await.retired);
        // Exiting while unused is not a failure: no backoff, and the restarter left it.
        assert_eq!(supervisor.state.lock().await.failures, 0);
        assert!(supervisor.is_running());

        // The next action starts a new worker with both factories, in order, and runs once.
        supervisor.request(invoke(&second, "order")).await.unwrap();
        assert_ne!(pid(&supervisor), original);
        assert!(!supervisor.state.lock().await.retired);
        supervisor.request(invoke(&first, "again")).await.unwrap();
        assert_eq!(
            fixture.actions(),
            ["first:used", "second:order", "first:again"]
        );
        // Each use moves the deadline, and the new worker exits again once unused.
        assert!(exited_within(&supervisor, Duration::from_secs(5)).await);
        supervisor.stop();
    }

    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn a_worker_with_work_of_its_own_stays_and_others_wake_only_when_used() {
        let idle = Duration::from_millis(300);
        let mut cases = Vec::new();
        for (name, source, stays) in [
            (
                "lifecycle",
                r#"import { onAppLaunch } from "chord"; export default () => { onAppLaunch("dev.fixture", () => {}); return () => {}; };"#,
                true,
            ),
            (
                "interval",
                "export default () => { setInterval(() => {}, 50); return () => {}; };",
                true,
            ),
            (
                "unref-interval",
                "export default () => { setInterval(() => {}, 50).unref(); return () => {}; };",
                false,
            ),
            ("quiet", "export default () => () => {};", false),
        ] {
            let fixture = Fixture::new();
            let supervisor = fixture.supervisor_exiting_after(idle).await;
            fixture.register_source(&supervisor, name, source).await;
            cases.push((name, fixture, supervisor, stays));
        }
        tokio::time::sleep(idle * 6).await;
        for (name, _, supervisor, stays) in &cases {
            assert_eq!(!has_exited(supervisor), *stays, "{name}");
        }
        for (name, _, supervisor, stays) in &cases {
            if *stays {
                continue;
            }
            // An app launching doesn't wake a worker without lifecycle callbacks.
            supervisor
                .request(Request::Lifecycle {
                    app: crate::app::desktop_app::ObservedApp {
                        pid: 123,
                        bundle_id: "dev.fixture".into(),
                    },
                    launch: true,
                })
                .await
                .unwrap();
            supervisor
                .request(Request::ReleaseSequences(vec![]))
                .await
                .unwrap();
            assert!(has_exited(supervisor), "{name} was woken");
            // Chord Mode does.
            supervisor.warm().await;
            assert!(!has_exited(supervisor), "{name} was not started again");
            supervisor.request(Request::Ping).await.unwrap();
        }
        for (_, _, supervisor, _) in &cases {
            supervisor.stop();
        }
    }
}
