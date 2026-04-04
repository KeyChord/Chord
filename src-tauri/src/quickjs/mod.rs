use crate::app::AppHandleExt;
use crate::app::chord_package_manager::PackageSpecifier;
use crate::app::desktop_app::clear_callbacks;
use crate::quickjs::chord_module::ChordModule;
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
use rquickjs::async_with;
use rquickjs::class::{Trace, Tracer};
use rquickjs::{
    AsyncContext, AsyncRuntime, CaughtError, Ctx, Error, JsLifetime, Module, Object, Value,
    loader::{Loader, Resolver},
    module::Declared,
};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, mpsc};
use std::thread;
use std::{cell::RefCell, future::Future, pin::Pin};
use tauri::AppHandle;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

mod chord_module;

struct JsEngine {
    // Keep the runtime alive for as long as the context exists.
    _rt: AsyncRuntime,
    ctx: AsyncContext,
}

thread_local! {
    static JS_ENGINE: RefCell<Option<JsEngine>> = RefCell::new(None);
    // static JS_WORKER: RefCell<Option<MainWorker>> = RefCell::new(None);
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
                .name("quickjs-worker".into())
                .spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("failed to build quickjs worker runtime");

                    while let Ok(task) = rx.recv() {
                        task(&runtime);
                    }
                })
                .expect("failed to spawn quickjs worker thread");

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
            .map_err(|_| anyhow::anyhow!("quickjs worker is unavailable"))?;

        rx.await
            .map_err(|_| anyhow::anyhow!("quickjs worker task dropped"))?
    }
}

pub struct AppUserData {
    pub handle: Option<AppHandle>,
}

// This tells rquickjs "this type does not contain JS references"
unsafe impl<'js> JsLifetime<'js> for AppUserData {
    type Changed<'to> = AppUserData;
}

// Usually safe because AppHandle doesn't hold JS values
impl<'js> Trace<'js> for AppUserData {
    fn trace(&self, _tracer: Tracer<'_, 'js>) {}
}

#[derive(Debug, Default)]
struct ModuleResolver {}

impl ModuleResolver {
    pub fn new() -> Self {
      Self {}
    }
}

impl Resolver for ModuleResolver {
    /// Our resolver ensures that modules within a js/ folder can ONLY access other JS files inside that
    /// folder. Thus, we need to have a list of all the modules inside a specific chord package's JS
    /// folder.
    ///
    /// To ensure consist module specifiers and avoid leaking implementation details in terms of
    /// where the package is stored on the filesystem, the module specifier of a JavaScript file
    /// in a chord package is always `name` + `relpath` (including /js/), e.g. `@keychord/chords-menu/js/menu.js`
    fn resolve<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        base_module_specifier: &str,
        import_specifier: &str,
    ) -> rquickjs::Result<String> {
        if import_specifier == "chord" {
            return Ok("chord".into());
        }

        // If we load it directly from memory, it should be already cached
        if base_module_specifier == "" {
            return Ok(import_specifier.into());
        }

        let specifier = PackageSpecifier::parse(base_module_specifier);
        let userdata = ctx.userdata::<AppUserData>();
        if let Some(userdata) = userdata {
            if let Some(handle) = &userdata.handle {
                let chord_pm = handle.chord_package_manager();
                let chord_package = chord_pm.get_package_by_name(specifier.package);
                if let Some(Some(js_package)) = chord_package.map(|p| p.js_package) {
                    if let Some(import) = js_package.resolve_import(&import_specifier) {
                        return Ok(import.clone())
                    }
                }
            }
        }

        Err(rquickjs::Error::new_resolving(base_module_specifier, import_specifier))
    }
}

#[derive(Debug, Default)]
struct ModuleLoader {}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {}
    }

    fn load_module<'js>(
        &self,
        ctx: &Ctx<'js>,
        name: &str,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        let module = match name {
            "chord" => Module::declare_def::<ChordModule, _>(ctx.clone(), "chord")?,
            _ => return Err(rquickjs::Error::new_loading_message("chord", "unable to load"))
        };

        Ok(module)
    }
}

fn attempted_module_path(name: &str) -> String {
    let path = Path::new(name);
    if path.is_absolute() {
        return path.display().to_string();
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path).display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn with_module_load_context(name: &str, error: Error) -> Error {
    match error {
        Error::Io(io_error) => Error::new_loading_message(
            name,
            format!(
                "tried to read {}: {}",
                attempted_module_path(name),
                io_error
            ),
        ),
        other => other,
    }
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js, Declared>> {
        let module = self.load_module(ctx, name)?;
        Ok(module)
    }
}

async fn build_engine(handle: Option<AppHandle>) -> anyhow::Result<JsEngine> {
    let rt = AsyncRuntime::new()?;
    rt.set_max_stack_size(1024 * 1024).await;
    let module_builder = llrt_modules::module_builder::ModuleBuilder::default()
        .with_global(llrt_core::modules::embedded::init)
        .with_global(llrt_core::builtins_inspect::init);
    let (llrt_module_resolver, llrt_module_loader, global_attachment) = module_builder.build();
    let chord_module_resolver = ModuleResolver::new();
    let resolver = (
        chord_module_resolver,
        llrt_module_resolver,
        llrt_core::modules::embedded::resolver::EmbeddedResolver,
        // llrt_core::modules::package::resolver::PackageResolver,
    );
    let chord_module_loader = ModuleLoader::new();
    let loader = (
        chord_module_loader,
        llrt_module_loader,
        llrt_core::modules::embedded::loader::EmbeddedLoader,
        // llrt_core::modules::package::loader::PackageLoader,
    );

    rt.set_loader(resolver, loader).await;

    let context = AsyncContext::full(&rt).await?;
    async_with!(context => |ctx| {
        async {
            global_attachment.attach(&ctx)?;
            ctx.store_userdata(AppUserData { handle })?;

            Ok::<_, Error>(())
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

#[allow(dead_code)]
pub async fn reset_js(handle: AppHandle) -> anyhow::Result<()> {
    JsWorker::global()
        .run(move |runtime| {
            runtime.block_on(async move {
                clear_callbacks();
                let engine = build_engine(Some(handle)).await?;
                JS_ENGINE.with(|cell| {
                    *cell.borrow_mut() = Some(engine);
                });

                Ok::<(), anyhow::Error>(())
            })
        })
        .await?;

    Ok(())
}

type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
type SendBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

unsafe fn uplift<'a, 'b, T>(fut: LocalBoxFuture<'a, T>) -> SendBoxFuture<'b, T> {
    unsafe { std::mem::transmute(fut) }
}

pub async fn with_js<F, R>(handle: AppHandle, f: F) -> anyhow::Result<R>
where
    F: Send + 'static + for<'js> FnOnce(Ctx<'js>) -> LocalBoxFuture<'js, anyhow::Result<R>>,
    R: Send + 'static,
{
    JsWorker::global()
        .run(move |runtime| {
            runtime.block_on(async move {
                let async_ctx: AsyncContext = ensure_engine(handle).await?;

                async_ctx
                    .async_with(|ctx| {
                        let fut = f(ctx.clone());
                        let fut = Box::pin(async move { fut.await });
                        unsafe { uplift(fut) }
                    })
                    .await
            })
        })
        .await
}

async fn import_module<'js>(ctx: Ctx<'js>, module_path: String) -> rquickjs::Result<()> {
    let module_promise = Module::import(&ctx, module_path)?;
    let _module = module_promise.into_future::<Object>().await?;
    Ok(())
}

async fn call_module_export<'js>(
    ctx: Ctx<'js>,
    module_path: String,
    export_name: String,
    args: Vec<serde_json::Value>,
) -> rquickjs::Result<()> {
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
        let value = rquickjs_serde::to_value(ctx.clone(), arg)
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

pub async fn run_standalone_module(path: &Path) -> anyhow::Result<()> {
    let module_path = canonicalize_module_path(path)?.display().to_string();
    let engine = build_engine(None).await?;

    engine
        .ctx
        .async_with(|ctx| {
            let module_path = module_path.clone();
            let fut = Box::pin(async move {
                import_module(ctx.clone(), module_path)
                    .await
                    .map_err(|error| anyhow::anyhow!(format_js_error(&ctx, error)))
            });
            unsafe { uplift(fut) }
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
    let engine = build_engine(None).await?;

    engine
        .ctx
        .async_with(|ctx| {
            let module_path = module_path.clone();
            let export_name = export_name.clone();
            let args = args.clone();
            let fut = Box::pin(async move {
                call_module_export(ctx.clone(), module_path, export_name, args)
                    .await
                    .map_err(|error| anyhow::anyhow!(format_js_error(&ctx, error)))
            });
            unsafe { uplift(fut) }
        })
        .await
}

pub fn format_js_error<'js>(ctx: &Ctx<'js>, error: Error) -> String {
    match CaughtError::from_error(ctx, error) {
        CaughtError::Error(error) => format!("Internal Engine Error: {error}"),
        CaughtError::Exception(exception) => format_js_exception(ctx, &exception),
        CaughtError::Value(value) => {
            // Force stringification of thrown primitives/objects
            let stringified = ctx.json_stringify(&value)
                .ok()
                .flatten()
                .and_then(|v| v.to_string().ok())
                .unwrap_or_else(|| format!("{:?}", value));
            format!("JavaScript threw a non-Error value: {}", stringified)
        },
    }
}

fn format_js_exception<'js>(ctx: &Ctx<'js>, exception: &rquickjs::Exception<'js>) -> String {
    let message = exception
        .message()
        .map(|msg| msg.trim().to_string())
        .filter(|msg| !msg.is_empty());

    let stack = exception
        .stack()
        .map(|stack| stack.trim().to_string())
        .filter(|stack| !stack.is_empty());

    match (message, stack) {
        (Some(message), Some(stack)) if stack.contains(&message) => stack,
        (Some(message), Some(stack)) => format!("{message}\n{stack}"),
        (Some(message), None) => {
            // No JS stack exists (Promise rejected with primitive, or reqwest error).
            // Dump the raw exception object.
            let obj = exception.as_value();
            let stringified = ctx.json_stringify(obj)
                .ok()
                .flatten()
                .and_then(|v| v.to_string().ok())
                .unwrap_or_else(|| format!("{:?}", obj));
            format!("{message}\n[No JS stack trace available] Raw object: {stringified}")
        },
        (None, Some(stack)) => stack,
        (None, None) => {
            let obj = exception.as_value();
            format!("Unknown exception without message or stack. Raw object: {:?}", obj)
        },
    }
}