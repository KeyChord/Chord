//! The Bun engine: the same shape as [`crate::quickjs`], on top of `rbun`.
//!
//! Everything runs on a dedicated `bun-worker` thread that owns the Bun VM
//! (JavaScriptCore is bound to the thread that created it) and a
//! current-thread tokio runtime for the async host code.

use crate::bun_js::chord_module::ChordModule;
use rbun::loader::{Loader, Resolver};
use rbun::module::Declared;
use rbun::prelude::*;
use rbun::{AsyncContext, AsyncRuntime, Module, RuntimeOptions, async_with};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, mpsc};
use std::thread;
use std::{cell::RefCell, future::Future, pin::Pin};
use tauri::{AppHandle, Manager};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

mod chord_module;
pub mod lifecycle;

struct JsEngine {
    // Keep the runtime alive for as long as the context exists.
    _rt: AsyncRuntime,
    ctx: AsyncContext,
}

thread_local! {
    static JS_ENGINE: RefCell<Option<JsEngine>> = const { RefCell::new(None) };
}

type JsTask = Box<dyn FnOnce(&Runtime) + Send + 'static>;

struct JsWorker {
    tx: mpsc::Sender<JsTask>,
}

impl JsWorker {
    fn global() -> &'static Self {
        static WORKER: OnceLock<JsWorker> = OnceLock::new();

        WORKER.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<JsTask>();

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

                    while let Ok(task) = rx.recv() {
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
            .map_err(|_| anyhow::anyhow!("bun worker is unavailable"))?;

        rx.await
            .map_err(|_| anyhow::anyhow!("bun worker task dropped"))?
    }
}

pub struct AppUserData {
    pub handle: Option<AppHandle>,
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
                return Err(rbun::Error::new_loading_message(
                    "chord",
                    "unable to load",
                ));
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

fn runtime_cwd(handle: Option<&AppHandle>) -> PathBuf {
    handle
        .and_then(|handle| handle.path().app_data_dir().ok())
        .filter(|dir| dir.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/"))
}

async fn build_engine(handle: Option<AppHandle>) -> anyhow::Result<JsEngine> {
    let rt = AsyncRuntime::new_with(RuntimeOptions {
        cwd: runtime_cwd(handle.as_ref()),
        argv: Some(vec!["chord".into()]),
        install_crash_handler: false,
    })?;
    rt.set_max_stack_size(1024 * 1024).await;
    rt.set_loader((ModuleResolver::new(),), (ModuleLoader::new(),))
        .await;

    let context = AsyncContext::full(&rt).await?;
    async_with!(context => |ctx| {
        async {
            ctx.store_userdata(AppUserData { handle })?;

            Ok::<_, rbun::Error>(())
        }.await.map_err(|e| anyhow::format_err!("async_with failed: {}", format_js_error(&ctx, e)))
    })
    .await?;

    Ok(JsEngine {
        _rt: rt,
        ctx: context,
    })
}

async fn ensure_engine(handle: AppHandle) -> anyhow::Result<AsyncContext> {
    let existing = JS_ENGINE.with(|cell| cell.borrow().as_ref().map(|engine| engine.ctx.clone()));
    if let Some(ctx) = existing {
        return Ok(ctx);
    }

    let engine = build_engine(Some(handle)).await?;
    let out = engine.ctx.clone();
    JS_ENGINE.with(|cell| {
        *cell.borrow_mut() = Some(engine);
    });

    Ok(out)
}

type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Run `f` with a [`Ctx`] on the Bun worker thread; the same contract as
/// [`crate::quickjs::with_js`].
pub async fn with_js<F, R>(handle: AppHandle, f: F) -> anyhow::Result<R>
where
    F: Send + 'static + for<'js> FnOnce(Ctx<'js>) -> LocalBoxFuture<'js, anyhow::Result<R>>,
    R: Send + 'static,
{
    JsWorker::global()
        .run(move |runtime| {
            runtime.block_on(async move {
                let async_ctx: AsyncContext = ensure_engine(handle).await?;

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
    let module_path = canonicalize_module_path(path)?.display().to_string();
    run_on_worker(move |runtime| {
        runtime.block_on(async move {
            let engine = build_engine(None).await?;
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
