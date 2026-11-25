//! Package code only executes in these supervised child processes, never in the GUI.
//! Supervisors replace failed workers and restore registrations; actions are never replayed.

use super::protocol::*;
use crate::app::AppHandleExt;
use anyhow::{Context, Result, ensure};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::process::{Child, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::{Notify, mpsc, oneshot};

const SETUP_TIMEOUT: Duration = Duration::from_secs(15);
const ACTION_TIMEOUT: Duration = Duration::from_secs(300);
const QUEUE_CAPACITY: usize = 64;

mod supervisor;
use supervisor::PackageSupervisor;

#[cfg(all(test, target_os = "macos"))]
#[path = "benchmarks.rs"]
mod benchmarks;

/// A loaded package. Its worker starts with the first JS handler it registers: most
/// packages declare their chords without running any JS, and each worker is a whole
/// Bun process, about 13 MB before the package's own code.
struct LoadedPackage {
    executable: PathBuf,
    config: WorkerConfig,
    handle: Option<AppHandle>,
    /// Set by the first start. A failed start is kept, so the package's other handlers
    /// fail at once instead of each waiting out the startup timeout; reloading retries.
    supervisor: tokio::sync::OnceCell<Result<Arc<PackageSupervisor>, String>>,
}

impl LoadedPackage {
    fn new(executable: PathBuf, config: WorkerConfig, handle: Option<AppHandle>) -> Arc<Self> {
        Arc::new(Self {
            executable,
            config,
            handle,
            supervisor: tokio::sync::OnceCell::new(),
        })
    }

    /// The same package, with no worker.
    fn idle(&self) -> Arc<Self> {
        Self::new(
            self.executable.clone(),
            self.config.clone(),
            self.handle.clone(),
        )
    }

    fn supervisor(&self) -> Option<Arc<PackageSupervisor>> {
        self.supervisor.get()?.as_ref().ok().cloned()
    }

    async fn start(&self, name: &str) -> Result<Arc<PackageSupervisor>> {
        self.supervisor
            .get_or_init(|| async {
                PackageSupervisor::spawn(
                    &self.executable,
                    name.to_string(),
                    self.config.clone(),
                    self.handle.clone(),
                    ACTION_TIMEOUT,
                    supervisor::IDLE_EXIT,
                )
                .await
                .map_err(|error| format!("{error:#}"))
            })
            .await
            .clone()
            .map_err(anyhow::Error::msg)
    }
}

fn packages() -> &'static Mutex<HashMap<String, Arc<LoadedPackage>>> {
    static PACKAGES: OnceLock<Mutex<HashMap<String, Arc<LoadedPackage>>>> = OnceLock::new();
    PACKAGES.get_or_init(Mutex::default)
}

/// The packages whose workers have started.
fn workers() -> Vec<(String, Arc<PackageSupervisor>)> {
    packages()
        .lock()
        .iter()
        .filter_map(|(name, package)| Some((name.clone(), package.supervisor()?)))
        .collect()
}

/// Replaces `name`'s entry with `next` (or removes it) only if it is still `current`,
/// so a package reloaded in the meantime keeps its new entry.
fn replace_package(name: &str, current: &Arc<LoadedPackage>, next: Option<Arc<LoadedPackage>>) {
    let mut packages = packages().lock();
    if !packages
        .get(name)
        .is_some_and(|package| Arc::ptr_eq(package, current))
    {
        return;
    }
    match next {
        Some(next) => packages.insert(name.to_string(), next),
        None => packages.remove(name),
    };
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

/// A worker that did not answer within the time it was given.
#[derive(Debug)]
struct Stalled(Duration);

impl std::fmt::Display for Stalled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no reply within {} seconds", self.0.as_secs())
    }
}

impl std::error::Error for Stalled {}

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
    /// Told when this worker fails or is stopped, so its owner can replace it.
    failed: Arc<Notify>,
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
        failed: Arc<Notify>,
    ) -> Result<Arc<Self>> {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let token = uuid::Uuid::new_v4().to_string();
        let address = listener.local_addr()?.to_string();
        #[cfg(target_os = "macos")]
        let mut command = {
            let mut command = Command::new(executable);
            command.args(["__package-worker", &address, &token]);
            command
        };
        #[cfg(not(target_os = "macos"))]
        let mut command = super::worker_command(executable, &address, &token)?;
        command
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
            failed: failed.clone(),
        });
        tokio::task::spawn_blocking(move || {
            let mut pressed = HashSet::new();
            let mut mouse = crate::native_input::MouseSession::default();
            while let Ok(request) = read_frame::<HostRequest>(&mut host) {
                let result = handle
                    .as_ref()
                    .context("Chord host APIs are unavailable")
                    .and_then(|handle| {
                        super::host::handle_host_request(handle, request, &mut pressed, &mut mouse)
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
                    let _ = super::host::handle_host_request(
                        &handle,
                        HostRequest::Release(key),
                        &mut HashSet::new(),
                        &mut mouse,
                    );
                }
            }
        });
        tokio::spawn(async move {
            // What ended the worker, when it was not a request that did.
            let mut ended = None;
            loop {
                // An idle worker owes no reply, so its socket only becomes readable
                // when the process has gone: the kernel closes it however it died.
                let mut unexpected = [0_u8; 1];
                let work = tokio::select! {
                    biased;
                    work = rx.recv() => match work {
                        Some(work) => work,
                        None => break,
                    },
                    read = control.peek(&mut unexpected) => {
                        ended = Some(match read {
                            Ok(0) => "closed its connection".to_string(),
                            Ok(_) => "sent a reply nobody asked for".to_string(),
                            Err(error) => format!("{error:#}"),
                        });
                        break;
                    }
                };
                let request = &work.request;
                let deadline =
                    if matches!(request, Request::Invoke { .. } | Request::Lifecycle { .. }) {
                        action_timeout
                    } else {
                        SETUP_TIMEOUT.min(action_timeout)
                    };
                // A handler may take minutes, so a worker stuck since its last request
                // (a package timer that never returns, say) must not be given that
                // long. The liveness check travels with the action and is answered
                // first, within the short deadline.
                let checked = deadline > SETUP_TIMEOUT;
                let prompt = SETUP_TIMEOUT.min(deadline);
                let exchange = async {
                    tokio::time::timeout(prompt, async {
                        if checked {
                            write_async(&mut control, &Request::Ping).await?;
                        }
                        write_async(&mut control, request).await?;
                        if checked {
                            read_async::<Response>(&mut control).await?;
                        }
                        Ok::<_, anyhow::Error>(())
                    })
                    .await
                    .map_err(|_| Stalled(prompt))??;
                    tokio::time::timeout(deadline, read_async::<Response>(&mut control))
                        .await
                        .map_err(|_| Stalled(deadline))?
                }
                .await;
                match exchange {
                    Ok(response) => {
                        let _ = work.reply.send(response.result.map_err(anyhow::Error::msg));
                    }
                    Err(error) => {
                        let reason = match error.downcast_ref::<Stalled>() {
                            Some(Stalled(deadline)) => format!(
                                "package {name} timed out after {} seconds",
                                deadline.as_secs()
                            ),
                            None => {
                                let status = process.child.lock().try_wait().ok().flatten();
                                format!(
                                    "package {name} worker exited or disconnected ({status:?}): {error:#}"
                                )
                            }
                        };
                        log::error!("{reason}; worker terminated");
                        *failure.lock() = Some(reason.clone());
                        let _ = work.reply.send(Err(anyhow::Error::msg(reason)));
                        break;
                    }
                }
            }
            if let Some(ended) = ended {
                let mut failure = failure.lock();
                // A worker stopped on purpose (reload, cancellation) is not news.
                if failure.is_none() {
                    let status = process.child.lock().try_wait().ok().flatten();
                    let reason = format!(
                        "package {name} worker exited or disconnected ({status:?}): {ended}"
                    );
                    log::error!("{reason}; worker terminated");
                    *failure = Some(reason);
                }
            }
            process.stop();
            failed.notify_one();
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
        self.failed.notify_one();
    }
}

/// Forgets the package's previous load and stops its worker. The next worker starts
/// when the package registers a handler.
pub async fn reset_package(handle: AppHandle, name: String, root: PathBuf) -> Result<()> {
    super::lifecycle::init_app_lifecycle(handle.clone());
    let cwd = handle
        .path()
        .app_data_dir()
        .ok()
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| root.clone());
    load_package(
        name,
        LoadedPackage::new(
            std::env::current_exe().context("failed to locate Chord")?,
            WorkerConfig { root, cwd },
            Some(handle),
        ),
    );
    Ok(())
}

fn load_package(name: String, package: Arc<LoadedPackage>) {
    let old = packages().lock().insert(name, package);
    if let Some(old) = old.and_then(|old| old.supervisor()) {
        old.stop();
    }
}

pub async fn register(handle: AppHandle, module: String, request: Request) -> Result<String> {
    let (name, _) = handle
        .app_state()
        .chord_package_manager()
        .package_for_path(Path::new(&module))
        .context("handler package is unavailable")?;
    register_in(&name, request).await
}

async fn register_in(name: &str, request: Request) -> Result<String> {
    let package = packages()
        .lock()
        .get(name)
        .cloned()
        .context("package is not loaded")?;
    let worker = package.start(name).await?;
    let id = worker.request(request).await?;
    let id = id
        .as_str()
        .context("invalid handler ID from package worker")?;
    Ok(format!("{}:{id}", worker.id))
}

fn worker_for_handler(id: &str) -> Result<(Arc<PackageSupervisor>, String)> {
    let (worker_id, handler_id) = id.split_once(':').context("invalid package handler ID")?;
    let worker = workers()
        .into_iter()
        .map(|(_, worker)| worker)
        .find(|worker| worker.id == worker_id)
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

/// Starts the workers that run these handlers again if they exited while unused, so the
/// chord being typed doesn't wait for them. Chord Mode passes the handlers that can run in
/// the frontmost app.
pub fn warm(handler_ids: HashSet<String>) {
    let worker_ids: HashSet<_> = handler_ids
        .iter()
        .filter_map(|id| id.split_once(':').map(|(worker, _)| worker.to_string()))
        .collect();
    let needed: Vec<_> = workers()
        .into_iter()
        .map(|(_, worker)| worker)
        .filter(|worker| worker_ids.contains(&worker.id))
        .collect();
    if needed.is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let mut jobs = tokio::task::JoinSet::new();
        for worker in needed {
            jobs.spawn(async move { worker.warm().await });
        }
        while jobs.join_next().await.is_some() {}
    });
}

pub async fn release_sequences(ids: Vec<String>) {
    // A dead package must not prevent healthy packages from dropping their sequence values.
    let mut jobs = tokio::task::JoinSet::new();
    for (_, worker) in workers() {
        let ids = ids.clone();
        jobs.spawn(async move {
            let _ = worker.request(Request::ReleaseSequences(ids)).await;
        });
    }
    while jobs.join_next().await.is_some() {}
}

pub async fn retain_handlers(ids: HashSet<String>, roots: HashMap<String, PathBuf>) {
    let snapshot: Vec<_> = packages()
        .lock()
        .iter()
        .map(|(name, package)| (name.clone(), package.clone()))
        .collect();
    for (name, package) in snapshot {
        if !roots.contains_key(&name) {
            replace_package(&name, &package, None);
            if let Some(worker) = package.supervisor() {
                worker.stop();
            }
            continue;
        }
        let Some(worker) = package.supervisor() else {
            continue;
        };
        let prefix = format!("{}:", worker.id);
        let ids: Vec<_> = ids
            .iter()
            .filter_map(|id| id.strip_prefix(&prefix).map(str::to_string))
            .collect();
        if ids.is_empty() {
            // No chord runs this package's JS any more (its handlers failed to load,
            // say), so its process goes. Registering a handler starts a new one.
            replace_package(&name, &package, Some(package.idle()));
            worker.stop();
            continue;
        }
        let _ = worker.request(Request::RetainHandlers(ids)).await;
    }
}

pub async fn dispatch_lifecycle(app: crate::app::desktop_app::ObservedApp, launch: bool) {
    let mut jobs = tokio::task::JoinSet::new();
    for (_, worker) in workers() {
        let app = app.clone();
        jobs.spawn(async move {
            if let Err(error) = worker.request(Request::Lifecycle { app, launch }).await {
                log::warn!("Package lifecycle callback failed: {error:#}");
            }
        });
    }
    while jobs.join_next().await.is_some() {}
}

/// Whether this package is loaded and its worker, if it needed one, started and hasn't
/// been stopped, so its registered handlers run.
pub fn is_running(name: &str) -> bool {
    packages().lock().get(name).is_some_and(|package| {
        package
            .supervisor
            .get()
            .is_none_or(|started| started.as_ref().is_ok_and(|worker| worker.is_running()))
    })
}

pub fn shutdown_all() {
    let packages: Vec<_> = packages()
        .lock()
        .drain()
        .map(|(_, package)| package)
        .collect();
    for worker in packages.iter().filter_map(|package| package.supervisor()) {
        worker.stop();
    }
}

#[cfg(target_os = "macos")]
pub struct HostClient(Mutex<TcpStream>);

#[cfg(target_os = "macos")]
impl HostClient {
    pub fn call(&self, request: HostRequest) -> Result<serde_json::Value> {
        let mut stream = self.0.lock();
        write_frame(&mut *stream, &request)?;
        let response: Response = read_frame(&mut *stream)?;
        response.result.map_err(anyhow::Error::msg)
    }
}

/// Entry point of the hidden CLI worker. The binary pumps the Cocoa main loop for Swift UI.
#[cfg(target_os = "macos")]
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
    // Keep one async context open, with Bun's event loop held open for as long as
    // the worker lives: the loop then serves the package's timers, sockets and
    // native add-on callbacks between requests, and sleeps when none is due.
    super::with_package_js(config, host, move |ctx| {
        Box::pin(async move {
            let mut keep_alive = Some(ctx.keep_event_loop_alive()?);
            while let Some(request) = incoming.recv().await {
                let result = if matches!(request, Request::HasOwnWork) {
                    has_work_of_its_own(&ctx, &mut keep_alive).map(serde_json::Value::Bool)
                } else {
                    super::execute_package_request(ctx.clone(), request).await
                };
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

/// Whether the package does anything between requests that exiting this worker while
/// it is unused would lose: it listens for apps launching or quitting, has a timer,
/// socket or subprocess that holds Bun's event loop open, or shows a window.
#[cfg(target_os = "macos")]
fn has_work_of_its_own<'js>(
    ctx: &rbun::Ctx<'js>,
    keep_alive: &mut Option<rbun::EventLoopKeepAlive<'js>>,
) -> Result<bool> {
    if super::lifecycle::has_callbacks() {
        return Ok(true);
    }
    // The worker's own timer holds the loop open; without it, is anything else?
    drop(keep_alive.take());
    let busy = ctx.is_event_loop_alive();
    *keep_alive = Some(ctx.keep_event_loop_alive()?);
    if busy {
        return Ok(true);
    }
    // Unsure counts as busy: a window closed under the person using it is worse than
    // a worker kept.
    Ok(crate::native_input::process_has_windows(std::process::id() as i32).unwrap_or(true))
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
                Arc::default(),
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

    /// How often the kernel has woken a process from idle since it started.
    #[cfg(target_os = "macos")]
    fn wakeups(pid: u32) -> u64 {
        // SAFETY: plain data, filled in by the call below.
        let mut info: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
        // SAFETY: the buffer matches the requested flavor.
        let status = unsafe {
            libc::proc_pid_rusage(
                pid as i32,
                libc::RUSAGE_INFO_V2,
                (&mut info as *mut libc::rusage_info_v2).cast(),
            )
        };
        assert_eq!(status, 0, "failed to sample process {pid}");
        info.ri_interrupt_wkups + info.ri_pkg_idle_wkups
    }

    /// A worker with production deadlines, where a liveness check travels with each action.
    async fn production_worker(fixture: &Fixture) -> Arc<PackageWorker> {
        let executable = PathBuf::from(
            std::env::var_os("CHORD_PACKAGE_TEST_HOST")
                .expect("set CHORD_PACKAGE_TEST_HOST to the freshly built chord executable"),
        );
        PackageWorker::spawn(
            &executable,
            "isolation-fixture".into(),
            WorkerConfig {
                root: fixture.0.clone(),
                cwd: fixture.0.clone(),
            },
            None,
            ACTION_TIMEOUT,
            Arc::default(),
        )
        .await
        .unwrap()
    }

    /// Nothing asks an idle worker whether it is alive, so nothing but the package's
    /// own timers may wake it, and those must run without a request to nudge them.
    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore = "takes 45 s and requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn an_idle_worker_sleeps_and_still_runs_the_packages_timers() {
        let fixture = Fixture::new();
        let quiet = production_worker(&fixture).await;
        let quiet_id = register(
            &quiet,
            fixture.module("quiet", "export default () => () => {};"),
        )
        .await
        .unwrap();
        quiet.request(invoke(&quiet_id, "warm")).await.unwrap();
        let pid = quiet.process.child.lock().id();
        // Bun collects garbage about once a second, on seven threads, for some 30 s
        // after the last activity, then stops: measured, 9 wake-ups a second, then 0.
        tokio::time::sleep(Duration::from_secs(40)).await;
        let before = wakeups(pid);
        tokio::time::sleep(Duration::from_secs(5)).await;
        let woken = wakeups(pid) - before;
        eprintln!("idle worker: {woken} wake-ups in 5 s");
        // Pinged every second with a 50 ms main-thread tick, this was over 100.
        assert!(woken < 20, "an idle worker woke {woken} times in 5 s");
        // Asleep, it still answers at once, and the liveness check goes with the action.
        tokio::time::timeout(
            Duration::from_secs(1),
            quiet.request(invoke(&quiet_id, "after-idle")),
        )
        .await
        .expect("an idle worker did not wake for a request")
        .unwrap();

        // A timer that does not hold the event loop open, like a package's
        // background refresh: it must keep running between requests.
        let ticks = fixture.0.join("ticks");
        let timers = production_worker(&fixture).await;
        let source = format!(
            r#"
let count = 0;
export default () => {{
    setInterval(() => {{ Bun.write({path}, String(++count)); }}, 20).unref();
    return () => {{}};
}};
"#,
            path = serde_json::to_string(&ticks).unwrap()
        );
        register(&timers, fixture.module("timers", &source))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(600)).await;
        let count: u32 = std::fs::read_to_string(&ticks)
            .expect("the package's timer never ran")
            .parse()
            .unwrap();
        assert!(
            count >= 5,
            "the package's timer ran {count} times in 600 ms"
        );
        quiet.stop();
        timers.stop();
    }

    /// The worker serves every request from one long poll of its task. Tokio's
    /// per-poll operation budget ran out after 64 of them and the worker went silent.
    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn a_worker_keeps_answering_past_tokios_operation_budget() {
        let fixture = Fixture::new();
        for worker in [fixture.worker().await, production_worker(&fixture).await] {
            let id = register(
                &worker,
                fixture.module("quiet", "export default () => () => {};"),
            )
            .await
            .unwrap();
            for request in 0..300 {
                worker
                    .request(invoke(&id, "again"))
                    .await
                    .unwrap_or_else(|error| panic!("request {request} failed: {error:#}"));
            }
            worker.stop();
        }
    }

    /// A worker stuck in package code while idle fails the next action within the
    /// setup deadline, not the minutes an action is allowed.
    #[tokio::test]
    #[ignore = "takes 15 s and requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn a_worker_stuck_while_idle_fails_the_next_action_promptly() {
        let fixture = Fixture::new();
        let worker = production_worker(&fixture).await;
        let id = register(
            &worker,
            fixture.module(
                "stuck",
                "export default () => { setTimeout(() => { while (true) {} }, 50); return () => {}; };",
            ),
        )
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        let started = std::time::Instant::now();
        let error = worker.request(invoke(&id, "any")).await.unwrap_err();
        assert!(
            error.to_string().contains("timed out after 15 seconds"),
            "{error:#}"
        );
        assert!(started.elapsed() < SETUP_TIMEOUT + Duration::from_secs(5));
        assert!(worker.request(Request::Ping).await.is_err());
        worker.stop();
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
            assert!(
                error.to_string().contains("has no addon for"),
                "attempt {attempt}: {error:#}"
            );
        }
        let healthy = fixture.module(
            "healthy-after",
            "export default function() { return () => {}; }",
        );
        register(&worker, healthy).await.unwrap();
    }

    /// Each worker is a whole Bun process, so a package has one only while a chord runs
    /// its JS. This is the only test that uses the package registry, which is global.
    #[tokio::test]
    #[ignore = "requires a freshly built chord executable; see tests/package-isolation/README.md"]
    async fn a_package_has_a_worker_only_while_its_handlers_are_in_use() {
        let fixture = Fixture::new();
        let executable = PathBuf::from(
            std::env::var_os("CHORD_PACKAGE_TEST_HOST")
                .expect("set CHORD_PACKAGE_TEST_HOST to the freshly built chord executable"),
        );
        let config = WorkerConfig {
            root: fixture.0.clone(),
            cwd: fixture.0.clone(),
        };
        let name = format!("lazy-fixture-{}", uuid::Uuid::new_v4());
        let roots = HashMap::from([(name.clone(), fixture.0.clone())]);
        let module = fixture.module("lazy", "export default () => () => {};");
        let registration = || Request::Register {
            module: module.clone(),
            raw: serde_json::json!({"name":"fixture"}),
            pathslug: "fixture.toml".into(),
            app_id: None,
            args: vec![],
        };
        let worker_of = |name: &str| {
            workers()
                .into_iter()
                .find(|(worker, _)| worker == name)
                .map(|(_, worker)| worker)
        };

        // Loading a package starts nothing.
        load_package(
            name.clone(),
            LoadedPackage::new(executable.clone(), config.clone(), None),
        );
        assert!(worker_of(&name).is_none());
        assert!(is_running(&name));

        // Its first handler starts the worker, which its other handlers share.
        let first = register_in(&name, registration()).await.unwrap();
        let second = register_in(&name, registration()).await.unwrap();
        let worker = worker_of(&name).expect("registering a handler started no worker");
        for id in [&first, &second] {
            assert!(id.starts_with(&format!("{}:", worker.id)), "{id}");
            let (owner, handler) = worker_for_handler(id).unwrap();
            owner.request(invoke(&handler, "run")).await.unwrap();
        }
        assert!(is_running(&name));

        // Keeping one handler keeps the worker; keeping none stops it, and the package
        // stays loaded for the next handler to start a new one.
        retain_handlers(HashSet::from([first.clone()]), roots.clone()).await;
        assert!(Arc::ptr_eq(&worker_of(&name).unwrap(), &worker));
        retain_handlers(HashSet::new(), roots.clone()).await;
        assert!(worker_of(&name).is_none());
        assert!(is_running(&name));
        let error = worker.request(Request::Ping).await.unwrap_err();
        assert!(error.to_string().contains("was unloaded"), "{error:#}");
        let third = register_in(&name, registration()).await.unwrap();
        let replacement = worker_of(&name).unwrap();
        assert!(!Arc::ptr_eq(&replacement, &worker));
        assert!(third.starts_with(&format!("{}:", replacement.id)));

        // A worker that cannot start fails the package's handlers, and the package does
        // not count as running, so the next reload loads it again.
        let broken = format!("broken-fixture-{}", uuid::Uuid::new_v4());
        load_package(
            broken.clone(),
            LoadedPackage::new(fixture.0.join("no-such-chord"), config, None),
        );
        for _ in 0..2 {
            let error = register_in(&broken, registration()).await.unwrap_err();
            assert!(
                error.to_string().contains("failed to start package worker"),
                "{error:#}"
            );
        }
        assert!(!is_running(&broken));

        // Unloading forgets the packages and stops their workers.
        retain_handlers(HashSet::new(), HashMap::new()).await;
        assert!(!is_running(&name) && !is_running(&broken));
        assert!(
            replacement
                .request(Request::Ping)
                .await
                .unwrap_err()
                .to_string()
                .contains("was unloaded")
        );
        let error = register_in(&name, registration()).await.unwrap_err();
        assert!(error.to_string().contains("not loaded"), "{error:#}");
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
