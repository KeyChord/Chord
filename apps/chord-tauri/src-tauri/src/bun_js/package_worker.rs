//! Package code only executes in these supervised child processes, never in the GUI.
//! Supervisors replace failed workers and restore registrations; actions are never replayed.

use super::protocol::*;
use crate::app::AppHandleExt;
use anyhow::{Context, Result, ensure};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, oneshot};

const SETUP_TIMEOUT: Duration = Duration::from_secs(15);
const ACTION_TIMEOUT: Duration = Duration::from_secs(300);
const HEALTH_INTERVAL: Duration = Duration::from_secs(1);
const QUEUE_CAPACITY: usize = 64;

mod supervisor;
use supervisor::PackageSupervisor;

#[cfg(all(test, target_os = "macos"))]
#[path = "benchmarks.rs"]
mod benchmarks;

fn workers() -> &'static Mutex<HashMap<String, Arc<PackageSupervisor>>> {
    static WORKERS: OnceLock<Mutex<HashMap<String, Arc<PackageSupervisor>>>> = OnceLock::new();
    WORKERS.get_or_init(Mutex::default)
}

struct Process {
    child: Mutex<Child>,
    sockets: Vec<TcpStream>,
}

impl Process {
    fn stop(&self) {
        for socket in &self.sockets {
            let _ = socket.shutdown(Shutdown::Both);
        }
        let _ = self.child.lock().kill();
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.stop();
        // The actor owns the final reference, so reaping never blocks the GUI thread.
        let _ = self.child.get_mut().wait();
    }
}

struct Work {
    request: Request,
    reply: oneshot::Sender<Result<serde_json::Value>>,
}

pub(crate) struct PackageWorker {
    pub id: String,
    name: String,
    tx: mpsc::Sender<Work>,
    process: Arc<Process>,
    failure: Arc<Mutex<Option<String>>>,
}

impl Drop for PackageWorker {
    fn drop(&mut self) {
        self.process.stop();
    }
}

impl PackageWorker {
    async fn spawn(
        executable: &Path,
        name: String,
        config: WorkerConfig,
        handle: Option<AppHandle>,
        action_timeout: Duration,
    ) -> Result<Arc<Self>> {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let token = uuid::Uuid::new_v4().to_string();
        let mut command = Command::new(executable);
        command
            .arg("__package-worker")
            .arg(listener.local_addr()?.to_string())
            .arg(&token)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // Package subprocess/job-control signals must not target the GUI's group.
            command.process_group(0);
        }
        let child = command.spawn().context("failed to start package worker")?;
        // Always kill/reap on handshake failure as well as on normal shutdown.
        let mut process = Process {
            child: Mutex::new(child),
            sockets: Vec::new(),
        };
        let handshake = tokio::time::timeout(SETUP_TIMEOUT, async {
            let (mut control, _) = listener.accept().await?;
            control.set_nodelay(true)?;
            let hello: Hello = read_async(&mut control).await?;
            ensure!(
                hello.token == token && hello.channel == "control",
                "invalid package worker handshake"
            );
            let (mut host, _) = listener.accept().await?;
            host.set_nodelay(true)?;
            let hello: Hello = read_async(&mut host).await?;
            ensure!(
                hello.token == token && hello.channel == "host",
                "invalid package host handshake"
            );
            write_async(&mut control, &config).await?;
            Ok::<_, anyhow::Error>((control, host))
        })
        .await
        .context("package worker startup timed out")??;
        drop(listener);
        let (control, host) = handshake;
        let control = control.into_std()?;
        process.sockets.push(control.try_clone()?);
        let mut host = host.into_std()?;
        host.set_nonblocking(false)?;
        process.sockets.push(host.try_clone()?);
        let process = Arc::new(process);
        let mut control = tokio::net::TcpStream::from_std(control)?;
        let (tx, mut rx) = mpsc::channel::<Work>(QUEUE_CAPACITY);
        let failure = Arc::new(Mutex::new(None));
        let worker = Arc::new(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.clone(),
            tx,
            process: process.clone(),
            failure: failure.clone(),
        });
        tokio::task::spawn_blocking(move || {
            let mut pressed = HashSet::new();
            let mut mouse = crate::native_input::MouseSession::default();
            while let Ok(request) = read_frame::<HostRequest>(&mut host) {
                let result = handle
                    .as_ref()
                    .context("Chord host APIs are unavailable")
                    .and_then(|handle| {
                        super::chord_module::handle_host_request(
                            handle,
                            request,
                            &mut pressed,
                            &mut mouse,
                        )
                    });
                let response = Response {
                    result: result.map_err(|error| format!("{error:#}")),
                };
                if write_frame(&mut host, &response).is_err() {
                    break;
                }
            }
            // A package dying after press() must not leave a simulated modifier held.
            mouse.release();
            if let Some(handle) = handle {
                for key in pressed {
                    let _ = super::chord_module::handle_host_request(
                        &handle,
                        HostRequest::Release(key),
                        &mut HashSet::new(),
                        &mut mouse,
                    );
                }
            }
        });
        tokio::spawn(async move {
            loop {
                let work = match tokio::time::timeout(HEALTH_INTERVAL, rx.recv()).await {
                    Ok(Some(work)) => Some(work),
                    Ok(None) => break,
                    Err(_) => None,
                };
                let request = work
                    .as_ref()
                    .map(|work| &work.request)
                    .unwrap_or(&Request::Ping);
                let deadline =
                    if matches!(request, Request::Invoke { .. } | Request::Lifecycle { .. }) {
                        action_timeout
                    } else {
                        SETUP_TIMEOUT.min(action_timeout)
                    };
                let exchange = tokio::time::timeout(deadline, async {
                    write_async(&mut control, request).await?;
                    read_async::<Response>(&mut control).await
                })
                .await;
                match exchange {
                    Ok(Ok(response)) => {
                        if let Some(work) = work {
                            let _ = work.reply.send(response.result.map_err(anyhow::Error::msg));
                        }
                    }
                    error => {
                        let status = process.child.lock().try_wait().ok().flatten();
                        let reason = match error {
                            Err(_) => format!(
                                "package {name} timed out after {} seconds",
                                deadline.as_secs()
                            ),
                            Ok(Err(error)) => format!(
                                "package {name} worker exited or disconnected ({status:?}): {error:#}"
                            ),
                            Ok(Ok(_)) => unreachable!(),
                        };
                        log::error!("{reason}; worker terminated");
                        *failure.lock() = Some(reason.clone());
                        process.stop();
                        if let Some(work) = work {
                            let _ = work.reply.send(Err(anyhow::Error::msg(reason)));
                        }
                        break;
                    }
                }
            }
            process.stop();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = process.child.lock().wait();
            })
            .await;
        });
        worker.request(Request::Ping).await?;
        Ok(worker)
    }

    pub(crate) async fn request(&self, request: Request) -> Result<serde_json::Value> {
        if let Some(failure) = self.failure.lock().clone() {
            anyhow::bail!("{failure}");
        }
        let (reply, result) = oneshot::channel();
        self.tx.try_send(Work { request, reply }).map_err(|_| {
            anyhow::anyhow!(
                "package {} worker is unavailable or its queue is full",
                self.name
            )
        })?;
        result.await.context("package worker stopped")?
    }

    fn stop(&self) {
        *self.failure.lock() = Some(format!("package {} worker stopped", self.name));
        self.process.stop();
    }
}

pub async fn reset_package(handle: AppHandle, name: String, root: PathBuf) -> Result<()> {
    let old = workers().lock().remove(&name);
    if let Some(old) = old {
        old.stop();
    }
    super::lifecycle::init_app_lifecycle(handle.clone());
    let cwd = handle
        .path()
        .app_data_dir()
        .ok()
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| root.clone());
    let worker = PackageSupervisor::spawn(
        &std::env::current_exe()?,
        name.clone(),
        WorkerConfig { root, cwd },
        Some(handle),
        ACTION_TIMEOUT,
    )
    .await?;
    workers().lock().insert(name, worker);
    Ok(())
}

pub async fn register(handle: AppHandle, module: String, request: Request) -> Result<String> {
    let (name, _) = handle
        .app_state()
        .chord_package_manager()
        .package_for_path(Path::new(&module))
        .context("handler package is unavailable")?;
    let worker = workers()
        .lock()
        .get(&name)
        .cloned()
        .context("package worker is unavailable")?;
    let id = worker.request(request).await?;
    let id = id
        .as_str()
        .context("invalid handler ID from package worker")?;
    Ok(format!("{}:{id}", worker.id))
}

fn worker_for_handler(id: &str) -> Result<(Arc<PackageSupervisor>, String)> {
    let (worker_id, handler_id) = id.split_once(':').context("invalid package handler ID")?;
    let worker = workers()
        .lock()
        .values()
        .find(|worker| worker.id == worker_id)
        .cloned()
        .context("package handler belongs to an unloaded configuration")?;
    Ok((worker, handler_id.to_string()))
}

pub async fn invoke(
    mut action: crate::models::HandlerChordAction,
    app_id: Option<String>,
    count: u32,
) -> Result<()> {
    let (worker, id) = worker_for_handler(&action.handler_id)?;
    action.handler_id = id;
    worker
        .request(Request::Invoke {
            action,
            app_id,
            count,
        })
        .await?;
    Ok(())
}

pub fn abort(handler_id: &str) {
    if let Ok((worker, _)) = worker_for_handler(handler_id) {
        worker.abort();
    }
}

pub async fn release_sequences(ids: Vec<String>) {
    let snapshot: Vec<_> = workers().lock().values().cloned().collect();
    // A dead package must not prevent healthy packages from dropping their sequence values.
    let mut jobs = tokio::task::JoinSet::new();
    for worker in snapshot {
        let ids = ids.clone();
        jobs.spawn(async move {
            let _ = worker.request(Request::ReleaseSequences(ids)).await;
        });
    }
    while jobs.join_next().await.is_some() {}
}

pub async fn retain_handlers(ids: HashSet<String>, roots: HashMap<String, PathBuf>) {
    let snapshot: Vec<_> = workers()
        .lock()
        .iter()
        .map(|(name, worker)| (name.clone(), worker.clone()))
        .collect();
    for (name, worker) in snapshot {
        if !roots.contains_key(&name) {
            worker.stop();
            workers().lock().remove(&name);
            continue;
        }
        let prefix = format!("{}:", worker.id);
        let ids = ids
            .iter()
            .filter_map(|id| id.strip_prefix(&prefix).map(str::to_string))
            .collect();
        let _ = worker.request(Request::RetainHandlers(ids)).await;
    }
}

pub async fn dispatch_lifecycle(app: crate::app::desktop_app::ObservedApp, launch: bool) {
    let snapshot: Vec<_> = workers().lock().values().cloned().collect();
    let mut jobs = tokio::task::JoinSet::new();
    for worker in snapshot {
        let app = app.clone();
        jobs.spawn(async move {
            if let Err(error) = worker.request(Request::Lifecycle { app, launch }).await {
                log::warn!("Package lifecycle callback failed: {error:#}");
            }
        });
    }
    while jobs.join_next().await.is_some() {}
}

pub fn shutdown_all() {
    for worker in workers().lock().drain().map(|(_, worker)| worker) {
        worker.stop();
    }
}

pub struct HostClient(Mutex<TcpStream>);

impl HostClient {
    pub fn call(&self, request: HostRequest) -> Result<serde_json::Value> {
        let mut stream = self.0.lock();
        write_frame(&mut *stream, &request)?;
        let response: Response = read_frame(&mut *stream)?;
        response.result.map_err(anyhow::Error::msg)
    }
}

/// Entry point of the hidden CLI worker. The binary pumps the Cocoa main loop for Swift UI.
pub async fn serve(address: String, token: String) -> Result<()> {
    let mut control = tokio::net::TcpStream::connect(&address).await?;
    control.set_nodelay(true)?;
    write_async(
        &mut control,
        &Hello {
            token: token.clone(),
            channel: "control".into(),
        },
    )
    .await?;
    let mut host = TcpStream::connect(&address)?;
    host.set_nodelay(true)?;
    host.set_read_timeout(Some(SETUP_TIMEOUT))?;
    host.set_write_timeout(Some(SETUP_TIMEOUT))?;
    write_frame(
        &mut host,
        &Hello {
            token,
            channel: "host".into(),
        },
    )?;
    let config: WorkerConfig = read_async(&mut control).await?;
    #[cfg(unix)]
    // A busy package is less important than keyboard capture and the GUI.
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
    }
    let host = Arc::new(HostClient(Mutex::new(host)));
    let std_control = control.into_std()?;
    let mut reader = tokio::net::TcpStream::from_std(std_control.try_clone()?)?;
    let mut control = tokio::net::TcpStream::from_std(std_control)?;
    let (requests, mut incoming) = mpsc::channel(8);
    tokio::spawn(async move {
        while let Ok(request) = read_async::<Request>(&mut reader).await {
            if requests.send(request).await.is_err() {
                return;
            }
        }
        // Exit even if a handler or native UI is still awaiting completion. This is only
        // the child process: an abruptly closed parent must not leave an orphan runtime.
        std::process::exit(0);
    });
    // Keep one async context open. rbun's driver waits on Bun's native I/O/timer loop
    // and the receiver's waker, so background timers and add-on callbacks run without
    // a polling timer or a new context acquisition every ten milliseconds.
    super::with_package_js(config, host, move |ctx| {
        Box::pin(async move {
            while let Some(request) = incoming.recv().await {
                let result = super::execute_package_request(ctx.clone(), request).await;
                write_async(
                    &mut control,
                    &Response {
                        result: result.map_err(|error| format!("{error:#}")),
                    },
                )
                .await?;
            }
            Ok(())
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChordKeyboardEvent, HandlerChordAction, Key, KeyEvent};
    use keycode::KeyMappingCode;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("chord-isolation-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(root.join("js")).unwrap();
            std::fs::write(root.join("package.json"), r#"{"name":"isolation-fixture"}"#).unwrap();
            Self(root)
        }

        fn module(&self, name: &str, source: &str) -> String {
            let path = self.0.join("js").join(format!("{name}.js"));
            std::fs::write(&path, source).unwrap();
            path.to_string_lossy().into_owned()
        }

        async fn worker(&self) -> Arc<PackageWorker> {
            let executable = PathBuf::from(
                std::env::var_os("CHORD_PACKAGE_TEST_HOST")
                    .expect("set CHORD_PACKAGE_TEST_HOST to the freshly built chord executable"),
            );
            PackageWorker::spawn(
                &executable,
                "isolation-fixture".into(),
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
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    async fn register(worker: &PackageWorker, module: String) -> Result<String> {
        let id = worker
            .request(Request::Register {
                module,
                raw: serde_json::json!({"name":"fixture"}),
                pathslug: "fixture.toml".into(),
                app_id: Some("dev.fixture".into()),
                args: vec![],
            })
            .await?;
        Ok(id.as_str().unwrap().to_string())
    }

    fn invoke(id: &str, mode: &str) -> Request {
        Request::Invoke {
            action: HandlerChordAction {
                handler_id: id.into(),
                event_args: vec![mode.into()],
                input: "hsr2".into(),
                sequence_id: Some("interval".into()),
                assign_to: Some("state".into()),
                keyboard_event: Some(ChordKeyboardEvent::new(
                    &KeyEvent::Press(Key(KeyMappingCode::Backspace)),
                    &[],
                    false,
                )),
                focused_window_id: None,
            },
            app_id: Some("dev.fixture".into()),
            count: 1,
        }
    }

    /// Uses the production supervisor and hidden CLI entry, including embedded Bun and FFI.
    /// These faults must only ever run in children, never in the Rust test process.
    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn package_faults_are_contained_and_healthy_workers_keep_running() {
        let fixture = Fixture::new();
        let healthy = fixture.worker().await;
        let healthy_pid = healthy.process.child.lock().id();
        let module = fixture.module("healthy", r#"
import { onAppLaunch, onAppTerminate, getGlobalHotkey } from "chord";
export default function() {
    if (this.chordsFilePath !== "fixture.toml" || this.chordsFileAppId !== "dev.fixture")
        throw new Error("factory context lost");
    if (!import.meta.chord.resolvePackageFile("target/probe").endsWith("/target/probe"))
        throw new Error("package root lost");
    if (!import.meta.chord.resolveNative("probe").endsWith("/probe/probe.node"))
        throw new Error("native resolver lost");
    globalThis.state = { count: 0, fn: () => 7 };
    globalThis.state.self = globalThis.state;
    globalThis.launched = false;
    globalThis.terminated = false;
    onAppLaunch("dev.fixture", () => { globalThis.launched = true; });
    const off = onAppTerminate("dev.fixture", () => { globalThis.terminated = true; });
    off();
    return async function(mode, previous) {
        if (mode === "throw") throw new Error("ordinary package exception");
        if (mode === "host-error") {
            try { getGlobalHotkey("dev.fixture", "fixture"); }
            catch (error) { if (String(error).includes("host request failed")) return; throw error; }
            throw new Error("expected unavailable host");
        }
        if (this.focusedAppId !== "dev.fixture" || !(this.event instanceof Event) ||
            this.event.key !== "Backspace") throw new Error("handler context lost");
        if (this.input !== "hsr2") throw new Error("chord input context lost");
        if (mode === "check" && (previous !== globalThis.state || previous.self !== previous ||
            previous.fn() !== 7)) throw new Error("object identity lost");
        if (mode === "lifecycle" && (!globalThis.launched || globalThis.terminated))
            throw new Error("lifecycle dispatch/unsubscribe lost");
        await Bun.sleep(1);
        globalThis.state.count++;
        return globalThis.state;
    };
}
"#);
        let id = register(&healthy, module).await.unwrap();
        healthy.request(invoke(&id, "prepare")).await.unwrap();
        let mut check = invoke(&id, "check");
        if let Request::Invoke { action, .. } = &mut check {
            action.event_args.push("$state".into());
        }
        healthy.request(check.clone()).await.unwrap();
        healthy.request(invoke(&id, "host-error")).await.unwrap();
        let error = healthy.request(invoke(&id, "throw")).await.unwrap_err();
        assert!(
            error.to_string().contains("ordinary package exception"),
            "{error:#}"
        );
        healthy.request(invoke(&id, "prepare")).await.unwrap();
        for launch in [true, false] {
            healthy
                .request(Request::Lifecycle {
                    app: crate::app::desktop_app::ObservedApp {
                        pid: 123,
                        bundle_id: "dev.fixture".into(),
                    },
                    launch,
                })
                .await
                .unwrap();
        }
        healthy.request(invoke(&id, "lifecycle")).await.unwrap();

        // Top-level imports and factories need the same boundary as handler invocation.
        for (name, source, request_fault) in [
            (
                "import-hang",
                "while (true) {} export default () => () => {};",
                false,
            ),
            (
                "factory-hang",
                "export default () => { while (true) {} };",
                false,
            ),
            (
                "import-abort",
                "process.abort(); export default () => () => {};",
                false,
            ),
            (
                "factory-exit",
                "export default () => { process.exit(42); };",
                false,
            ),
            (
                "handler-abort",
                "export default () => () => process.abort();",
                true,
            ),
            (
                "handler-segv",
                r#"import { dlopen, FFIType } from "bun:ffi"; const native = dlopen("/usr/lib/libSystem.B.dylib", { memset: { args: [FFIType.ptr, FFIType.i32, FFIType.usize], returns: FFIType.ptr } }); export default () => () => native.symbols.memset(null, 0, 16);"#,
                true,
            ),
            (
                "process-group-abort",
                // Signal delivery can race a successful IPC reply. Keep the handler
                // pending so the assertion observes the exit rather than that race.
                r#"import { dlopen, FFIType } from "bun:ffi"; const native = dlopen("/usr/lib/libSystem.B.dylib", { getpgrp: { args: [], returns: FFIType.i32 } }); export default () => () => { if (native.symbols.getpgrp() !== process.pid) throw new Error("worker lacks process-group isolation"); process.kill(0, "SIGABRT"); return new Promise(() => {}); };"#,
                true,
            ),
            (
                "handler-hang",
                "export default () => () => { while (true) {} };",
                true,
            ),
            (
                "promise-hang",
                "export default () => () => new Promise(() => {});",
                true,
            ),
            (
                "lifecycle-abort",
                r#"import { onAppLaunch } from "chord"; export default () => { onAppLaunch("dev.fixture", () => process.abort()); return () => {}; };"#,
                false,
            ),
            (
                "timer-abort",
                "export default () => { setTimeout(() => process.abort(), 50); return () => {}; };",
                false,
            ),
        ] {
            let broken = fixture.worker().await;
            let registration = register(&broken, fixture.module(name, source)).await;
            let result = if request_fault {
                let request = invoke(&registration.unwrap(), "fault");
                let (fault, healthy_result) = tokio::join!(
                    broken.request(request),
                    tokio::time::timeout(
                        Duration::from_millis(250),
                        healthy.request(check.clone())
                    ),
                );
                healthy_result
                    .expect("healthy worker was blocked by another package")
                    .unwrap();
                fault
            } else if name == "lifecycle-abort" {
                registration.unwrap();
                broken
                    .request(Request::Lifecycle {
                        app: crate::app::desktop_app::ObservedApp {
                            pid: 123,
                            bundle_id: "dev.fixture".into(),
                        },
                        launch: true,
                    })
                    .await
            } else if name == "timer-abort" {
                registration.unwrap();
                tokio::time::sleep(Duration::from_millis(150)).await;
                broken.request(Request::Ping).await
            } else {
                registration.map(serde_json::Value::String)
            };
            assert!(result.is_err(), "{name} unexpectedly survived");
            assert!(
                broken.request(Request::Ping).await.is_err(),
                "{name} silently restarted"
            );
            healthy.request(check.clone()).await.unwrap();
            assert_eq!(healthy.process.child.lock().id(), healthy_pid);
            broken.stop();
        }

        let replacement = fixture.worker().await;
        assert_ne!(replacement.id, healthy.id);
        let replacement_id = register(
            &replacement,
            fixture.module("replacement", "export default () => () => {};"),
        )
        .await
        .unwrap();
        replacement
            .request(invoke(&replacement_id, "healthy"))
            .await
            .unwrap();
        replacement.stop();
        healthy.stop();
    }

    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn a_handler_that_throws_while_loading_fails_fast_every_time() {
        let fixture = Fixture::new();
        // The fixture's deadline is 500 ms, so a stalled registration shows up as a timeout.
        let worker = fixture.worker().await;
        // What an add-on built only for another architecture does, as `@keychord/config` builds it.
        let missing = fixture.module("missing-addon", r#"
const nativeFiles = { "plan9-mips": "native/plan9-mips/menu/menu.node" };
const nativeTarget = process.platform + "-" + process.arch;
if (!nativeFiles[nativeTarget]) throw new Error("[chord-native] menu has no addon for " + nativeTarget);
export default function() { return () => {}; }
"#);
        // A reload imports the same module again.
        for attempt in 1..=2 {
            let error = register(&worker, missing.clone()).await.unwrap_err();
            assert!(error.to_string().contains("has no addon for"), "attempt {attempt}: {error:#}");
        }
        let healthy = fixture.module("healthy-after", "export default function() { return () => {}; }");
        register(&worker, healthy).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires legacy NodeSwift fixtures; see tests/package-isolation/README.md"]
    async fn legacy_node_swift_fatal_error_is_contained() {
        let native = PathBuf::from(
            std::env::var_os("CHORD_LEGACY_NATIVE_FIXTURE")
                .expect("set CHORD_LEGACY_NATIVE_FIXTURE to the fixture's build directory"),
        );
        let fixture = Fixture::new();
        let healthy = fixture.worker().await;
        let broken = fixture.worker().await;
        let healthy_id = register(
            &healthy,
            fixture.module("healthy-native-peer", "export default () => () => {};"),
        )
        .await
        .unwrap();
        let paths = serde_json::to_string(&[native.join("First.node"), native.join("Second.node")])
            .unwrap();
        let source = format!(
            r#"
const [first, second] = {paths}.map(path => {{
    const module = {{ exports: {{}} }};
    process.dlopen(module, path);
    return module.exports;
}});
second.noop();
export default () => async () => {{
    for (let i = 0; i < 100; i++) await first.hop();
}};
"#
        );
        let id = register(&broken, fixture.module("legacy-node-swift", &source))
            .await
            .unwrap();
        let error = broken
            .request(invoke(&id, "native-fault"))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("exited or disconnected"),
            "{error:#}"
        );
        healthy
            .request(invoke(&healthy_id, "healthy"))
            .await
            .unwrap();
        broken.stop();
        healthy.stop();
    }
}
