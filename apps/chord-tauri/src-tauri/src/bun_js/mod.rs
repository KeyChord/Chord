//! The embedded Bun runtime, exposed through `rbun`.
//!
//! In package child processes and standalone CLI invocations, a dedicated
//! `bun-worker` thread owns the Bun VM
//! (JavaScriptCore is bound to the thread that created it) and a
//! current-thread tokio runtime for the async host code.

use crate::bun_js::chord_module::ChordModule;
use rbun::loader::{Loader, Resolver};
use rbun::module::Declared;
use rbun::prelude::*;
use rbun::{AsyncContext, AsyncRuntime, Module, RuntimeOptions, async_with};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread;
use std::{cell::RefCell, future::Future, pin::Pin};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};

mod chord_module;
mod host;
pub mod lifecycle;
pub(crate) mod package_worker;
pub(crate) mod protocol;
mod registration;

struct BunEngine {
    // Keep the runtime alive for as long as the context exists.
    _rt: AsyncRuntime,
    ctx: AsyncContext,
}

thread_local! {
    static BUN_ENGINE: RefCell<Option<BunEngine>> = const { RefCell::new(None) };
}

type JsTask = Box<dyn FnOnce(&Runtime) + Send + 'static>;
const JS_WORKER_QUEUE_CAPACITY: usize = 64;

struct JsWorker {
    tx: mpsc::Sender<JsTask>,
}

impl JsWorker {
    fn global() -> &'static Self {
        static WORKER: OnceLock<JsWorker> = OnceLock::new();

        WORKER.get_or_init(|| {
            let (tx, mut rx) = mpsc::channel::<JsTask>(JS_WORKER_QUEUE_CAPACITY);

            thread::Builder::new()
                .name("bun-worker".into())
                // JavaScriptCore sizes its JS stack from the host thread; the
                // CLI gives its main thread 18 MB.
                .stack_size(16 * 1024 * 1024)
                .spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("failed to build bun worker runtime");

                    while let Some(task) = rx.blocking_recv() {
                        task(&runtime);
                    }
                })
                .expect("failed to spawn bun worker thread");

            Self { tx }
        })
    }

    async fn run<R, F>(&self, task: F) -> anyhow::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&Runtime) -> anyhow::Result<R> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        self.tx
            .send(Box::new(move |runtime| {
                let _ = tx.send(task(runtime));
            }))
            .await
            .map_err(|_| anyhow::anyhow!("bun worker is unavailable"))?;

        rx.await
            .map_err(|_| anyhow::anyhow!("bun worker task dropped"))?
    }
}

#[derive(Clone)]
pub(crate) struct PackageContext {
    pub config: protocol::WorkerConfig,
    pub host: std::sync::Arc<package_worker::HostClient>,
}

pub struct AppUserData {
    pub(crate) package: Option<PackageContext>,
}

// This tells rbun "this type does not contain JS references"
unsafe impl<'js> JsLifetime<'js> for AppUserData {
    type Changed<'to> = AppUserData;
}

#[derive(Debug, Default)]
struct ModuleResolver {}

impl ModuleResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Resolver for ModuleResolver {
    /// Only the built-in `chord` module is resolved here. Package modules are
    /// imported from disk by absolute path, so everything else — relative
    /// imports between package files, `node:*`, `bun:*`,
    /// `node_modules` shipped by a package — falls through to Bun's own
    /// resolution.
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base_module_specifier: &str,
        import_specifier: &str,
    ) -> rbun::Result<String> {
        if import_specifier == "chord" {
            return Ok("chord".into());
        }

        Err(rbun::Error::new_resolving(
            base_module_specifier,
            import_specifier,
        ))
    }
}

#[derive(Debug, Default)]
struct ModuleLoader {}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {}
    }

    fn load_module<'js>(&self, ctx: &Ctx<'js>, name: &str) -> rbun::Result<Module<'js, Declared>> {
        let module = match name {
            "chord" => Module::declare_def::<ChordModule, _>(ctx.clone(), "chord")?,
            _ => {
                return Err(rbun::Error::new_loading_message("chord", "unable to load"));
            }
        };

        Ok(module)
    }
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rbun::Result<Module<'js, Declared>> {
        let module = self.load_module(ctx, name)?;
        Ok(module)
    }
}

fn runtime_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
}

async fn build_engine(package: Option<PackageContext>) -> anyhow::Result<BunEngine> {
    let rt = AsyncRuntime::new_with(RuntimeOptions {
        cwd: package
            .as_ref()
            .map(|package| package.config.cwd.clone())
            .unwrap_or_else(|| runtime_cwd()),
        argv: Some(vec!["chord".into()]),
        install_crash_handler: false,
    })?;
    rt.set_max_stack_size(1024 * 1024).await;
    rt.set_loader((ModuleResolver::new(),), (ModuleLoader::new(),))
        .await;
    // Attach `import.meta.chord` to every module loaded from disk. Installed before any
    // handler is imported, since JSC materializes a module's `import.meta` once.
    chord_module::install_import_meta_hook(&rt).await;

    let context = AsyncContext::full(&rt).await?;
    async_with!(context => |ctx| {
        async {
            ctx.store_userdata(AppUserData { package })?;

            Ok::<_, rbun::Error>(())
        }.await.map_err(|e| anyhow::format_err!("async_with failed: {}", format_js_error(&ctx, e)))
    })
    .await?;

    Ok(BunEngine {
        _rt: rt,
        ctx: context,
    })
}

async fn ensure_engine(package: Option<PackageContext>) -> anyhow::Result<AsyncContext> {
    let existing = BUN_ENGINE.with(|cell| cell.borrow().as_ref().map(|engine| engine.ctx.clone()));
    if let Some(ctx) = existing {
        return Ok(ctx);
    }

    let engine = build_engine(package).await?;
    let out = engine.ctx.clone();
    BUN_ENGINE.with(|cell| {
        *cell.borrow_mut() = Some(engine);
    });

    Ok(out)
}

type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Run `f` with a [`Ctx`] on the Bun worker thread.
pub(crate) async fn with_package_js<F, R>(
    config: protocol::WorkerConfig,
    host: std::sync::Arc<package_worker::HostClient>,
    f: F,
) -> anyhow::Result<R>
where
    F: Send + 'static + for<'js> FnOnce(Ctx<'js>) -> LocalBoxFuture<'js, anyhow::Result<R>>,
    R: Send + 'static,
{
    JsWorker::global()
        .run(move |runtime| {
            runtime.block_on(async move {
                let async_ctx: AsyncContext =
                    ensure_engine(Some(PackageContext { config, host })).await?;

                async_ctx.async_with(|ctx| f(ctx)).await
            })
        })
        .await
}

async fn import_module<'js>(ctx: Ctx<'js>, module_path: String) -> rbun::Result<()> {
    let module_promise = Module::import(&ctx, module_path)?;
    let _module = module_promise.into_future::<Object>().await?;
    Ok(())
}

async fn call_module_export<'js>(
    ctx: Ctx<'js>,
    module_path: String,
    export_name: String,
    args: Vec<serde_json::Value>,
) -> rbun::Result<()> {
    let module_promise = Module::import(&ctx, module_path)?;
    let module = module_promise.into_future::<Object>().await?;

    let mut export: Value<'js> = module.get(export_name.clone())?;
    if let Some(promise) = export.as_promise().cloned() {
        export = promise.into_future::<Value<'js>>().await?;
    }

    let function = export.as_function().cloned().or_throw_msg(
        &ctx,
        &format!(
            "JS export `{}` did not resolve to a function: {:?}",
            export_name, export
        ),
    )?;

    let mut args_builder = Args::new(ctx.clone(), args.len());
    for arg in args {
        let value = rbun::serde::to_value(ctx.clone(), arg)
            .or_throw_msg(&ctx, "Failed to convert CLI arguments")?;
        args_builder.push_arg(value)?;
    }

    let mut result: Value<'js> = function.call_arg(args_builder)?;
    if let Some(promise) = result.as_promise().cloned() {
        result = promise.into_future::<Value<'js>>().await?;
    }

    let _ = result;
    Ok(())
}

fn canonicalize_module_path(path: &Path) -> anyhow::Result<PathBuf> {
    let path = std::fs::canonicalize(path)?;
    if !path.is_file() {
        anyhow::bail!("expected a JavaScript file path, got {}", path.display());
    }

    Ok(path)
}

fn run_on_worker<R, F>(f: F) -> impl Future<Output = anyhow::Result<R>>
where
    R: Send + 'static,
    F: FnOnce(&Runtime) -> anyhow::Result<R> + Send + 'static,
{
    JsWorker::global().run(f)
}

pub async fn run_standalone_module(path: &Path) -> anyhow::Result<()> {
    run_standalone_module_with_args(path, Vec::new()).await
}

pub async fn run_standalone_module_with_args(path: &Path, args: Vec<String>) -> anyhow::Result<()> {
    let module_path = canonicalize_module_path(path)?.display().to_string();
    run_on_worker(move |runtime| {
        runtime.block_on(async move {
            let engine = build_engine(None).await?;
            engine
                ._rt
                .inner()
                .configure_entrypoint(&module_path, &args)?;
            engine
                .ctx
                .async_with(|ctx| {
                    let module_path = module_path.clone();
                    Box::pin(async move {
                        import_module(ctx.clone(), module_path)
                            .await
                            .map_err(|error| anyhow::anyhow!(format_js_error(&ctx, error)))
                    })
                })
                .await?;
            // Let timers / servers started by the script finish, like `bun run`.
            engine.ctx.with(|ctx| ctx.run_until_idle()).await;
            Ok(())
        })
    })
    .await
}

#[cfg(test)]
pub(crate) async fn with_test_js<F, R>(f: F) -> anyhow::Result<R>
where
    F: Send + 'static + for<'js> FnOnce(Ctx<'js>) -> LocalBoxFuture<'js, anyhow::Result<R>>,
    R: Send + 'static,
{
    JsWorker::global()
        .run(move |runtime| {
            runtime.block_on(async move { ensure_engine(None).await?.async_with(f).await })
        })
        .await
}

fn parse_cli_arg(arg: String) -> serde_json::Value {
    serde_json::from_str(&arg).unwrap_or(serde_json::Value::String(arg))
}

pub async fn run_standalone_export(
    path: &Path,
    export_name: String,
    args: Vec<String>,
) -> anyhow::Result<()> {
    let module_path = canonicalize_module_path(path)?.display().to_string();
    let args: Vec<serde_json::Value> = args.into_iter().map(parse_cli_arg).collect();
    run_on_worker(move |runtime| {
        runtime.block_on(async move {
            let engine = build_engine(None).await?;
            engine
                .ctx
                .async_with(|ctx| {
                    let module_path = module_path.clone();
                    let export_name = export_name.clone();
                    let args = args.clone();
                    Box::pin(async move {
                        call_module_export(ctx.clone(), module_path, export_name, args)
                            .await
                            .map_err(|error| anyhow::anyhow!(format_js_error(&ctx, error)))
                    })
                })
                .await?;
            engine.ctx.with(|ctx| ctx.run_until_idle()).await;
            Ok(())
        })
    })
    .await
}

pub fn format_js_error<'js>(ctx: &Ctx<'js>, error: rbun::Error) -> String {
    rbun::format_error(ctx, error)
}

async fn execute_package_request(
    ctx: Ctx<'_>,
    request: protocol::Request,
) -> anyhow::Result<serde_json::Value> {
    use protocol::Request;
    let result = async {
        match request {
            Request::Register {
                module,
                raw,
                pathslug,
                app_id,
                args,
            } => {
                let id = registration::register(ctx.clone(), module, raw, pathslug, app_id, args)
                    .await?;
                return Ok(serde_json::Value::String(id));
            }
            Request::Invoke {
                action,
                app_id,
                count,
            } => {
                crate::app::chord_runner::invoke_js_handler(ctx.clone(), &action, app_id, count)
                    .await?;
            }
            Request::ReleaseSequences(ids) => {
                crate::app::chord_runner::release_sequence_values(ctx.clone(), ids)?
            }
            Request::RetainHandlers(ids) => {
                let ids: std::collections::HashSet<_> = ids.into_iter().collect();
                if let Some(registry) = ctx
                    .globals()
                    .get::<_, Option<Object>>("__RUST_HANDLER_REGISTRY")?
                {
                    for key in registry
                        .keys::<String>()
                        .collect::<rbun::Result<Vec<_>>>()?
                    {
                        if !ids.contains(&key) {
                            registry.remove(key)?;
                        }
                    }
                }
            }
            Request::Lifecycle { app, launch } => {
                // Lifecycle helpers already return anyhow errors with callback context.
                return if launch {
                    lifecycle::invoke_launch_callbacks(ctx.clone(), app).await
                } else {
                    lifecycle::invoke_terminate_callbacks(ctx.clone(), app).await
                }
                .map(|_| serde_json::Value::Null);
            }
            // Answering shows the VM's thread is free; the worker's event loop runs
            // on its own (see `package_worker::serve`).
            Request::Ping => {}
        }
        Ok::<_, anyhow::Error>(serde_json::Value::Null)
    }
    .await;
    result.map_err(|error| match error.downcast::<rbun::Error>() {
        Ok(js_error) => anyhow::anyhow!(format_js_error(&ctx, js_error)),
        Err(error) => error,
    })
}
