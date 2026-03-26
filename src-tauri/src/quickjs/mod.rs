use crate::app::desktop_app::clear_callbacks;
use crate::quickjs::chord_module::ChordModule;
use llrt_readline::{ReadlineModule, ReadlinePromisesModule};
use rquickjs::async_with;
use rquickjs::class::{Trace, Tracer};
#[allow(unused_imports)]
use rquickjs::{
    AsyncContext, AsyncRuntime, CaughtError, Ctx, Error, Function, JsLifetime, Module, Object,
    Value,
    loader::{Loader, Resolver},
    module::Declared,
};
use std::path::Path;
use std::{cell::RefCell, future::Future, pin::Pin};
use tauri::{
    AppHandle,
    async_runtime::{block_on, channel},
};

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

pub struct AppUserData {
    pub handle: AppHandle,
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
struct ModuleResolver {
    llrt_resolver: llrt_modules::module::resolver::ModuleResolver,
}

impl ModuleResolver {
    pub fn new(llrt_resolver: llrt_modules::module::resolver::ModuleResolver) -> Self {
        Self { llrt_resolver }
    }
}

impl Resolver for ModuleResolver {
    fn resolve<'js>(&mut self, ctx: &Ctx<'js>, base: &str, name: &str) -> rquickjs::Result<String> {
        // `.` from `.js`
        if name.contains(".") || name == "chord" {
            return Ok(name.into());
        }

        if name == "readline"
            || name == "node:readline"
            || name == "readline/promises"
            || name == "node:readline/promises"
        {
            return Ok("readline".into());
        }

        self.llrt_resolver.resolve(ctx, base, name)
    }
}

#[derive(Debug, Default)]
struct ModuleLoader {
    llrt_loader: llrt_modules::module::loader::ModuleLoader,
}

impl ModuleLoader {
    pub fn new(llrt_loader: llrt_modules::module::loader::ModuleLoader) -> Self {
        Self { llrt_loader }
    }
}

fn get_module<'js>(ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Option<Module<'js, Declared>>> {
    let module = match name {
        "chord" => Module::declare_def::<ChordModule, _>(ctx.clone(), "chord"),
        "readline" | "node:readline" => {
            Module::declare_def::<ReadlineModule, _>(ctx.clone(), "readline")
        }
        "readline/promises" | "node:readline/promises" => {
            Module::declare_def::<ReadlinePromisesModule, _>(ctx.clone(), "readline/promises")
        }
        _ => return Ok(None),
    };

    Some(module).transpose()
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
            format!("tried to read {}: {}", attempted_module_path(name), io_error),
        ),
        other => other,
    }
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js, Declared>> {
        let module = get_module(ctx, name)?;
        Ok(match module {
            Some(module) => module,
            None => self
                .llrt_loader
                .load(ctx, name)
                .map_err(|error| with_module_load_context(name, error))?,
        })
    }
}

async fn ensure_engine(handle: AppHandle) -> anyhow::Result<AsyncContext> {
    let existing = JS_ENGINE.with(|cell| cell.borrow().as_ref().map(|engine| engine.ctx.clone()));
    if let Some(ctx) = existing {
        return Ok(ctx);
    }

    let rt = AsyncRuntime::new()?;
    let module_builder = llrt_modules::module_builder::ModuleBuilder::default()
        .with_global(llrt_core::modules::embedded::init)
        .with_global(llrt_core::builtins_inspect::init);
    let (llrt_module_resolver, llrt_module_loader, global_attachment) = module_builder.build();
    let module_resolver = ModuleResolver::new(llrt_module_resolver);
    let resolver = (
        module_resolver,
        llrt_core::modules::embedded::resolver::EmbeddedResolver,
        llrt_core::modules::package::resolver::PackageResolver,
    );
    let module_loader = ModuleLoader::new(llrt_module_loader);
    let loader = (
        module_loader,
        llrt_core::modules::embedded::loader::EmbeddedLoader,
        llrt_core::modules::package::loader::PackageLoader,
    );

    rt.set_loader(resolver, loader).await;

    let context = AsyncContext::full(&rt).await?;
    async_with!(context => |ctx| {
        global_attachment.attach(&ctx)?;
        ctx.store_userdata(AppUserData { handle })?;

        Ok::<_, Error>(())
    })
    .await?;

    // Deno makes the app super slow
    // JS_WORKER.with(move |cell| {
    //     *cell.borrow_mut() = Some(main_worker);
    // });

    let out = context.clone();
    JS_ENGINE.with(|cell| {
        *cell.borrow_mut() = Some(JsEngine {
            _rt: rt,
            ctx: context,
        });
    });

    Ok(out)
}

pub async fn reset_js(handle: AppHandle) -> anyhow::Result<()> {
    let (tx, mut rx) = channel(1);

    handle.run_on_main_thread(move || {
        clear_callbacks();
        JS_ENGINE.with(|cell| {
            *cell.borrow_mut() = None;
        });

        let _ = tx.try_send(Ok::<(), anyhow::Error>(()));
    })?;

    rx.recv()
        .await
        .ok_or_else(|| anyhow::anyhow!("main thread task dropped"))??;

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
    let (tx, mut rx) = channel(1);

    handle.clone().run_on_main_thread(move || {
        let result = block_on(async move {
            let async_ctx: AsyncContext = ensure_engine(handle).await?;

            async_ctx
                .async_with(|ctx| {
                    let fut = f(ctx.clone());
                    let fut = Box::pin(async move { fut.await });
                    unsafe { uplift(fut) }
                })
                .await
        });

        let _ = tx.try_send(result);
    })?;

    rx.recv()
        .await
        .ok_or_else(|| anyhow::anyhow!("main thread task dropped"))?
}

pub fn format_js_error<'js>(ctx: &Ctx<'js>, error: Error) -> String {
    match CaughtError::from_error(ctx, error) {
        CaughtError::Error(error) => error.to_string(),
        CaughtError::Exception(exception) => format_js_exception(&exception),
        CaughtError::Value(value) => format!("JavaScript threw a non-Error value: {:?}", value),
    }
}

fn format_js_exception<'js>(exception: &rquickjs::Exception<'js>) -> String {
    let message = exception
        .message()
        .map(|message| message.trim().to_string())
        .filter(|message| !message.is_empty());
    let stack = exception
        .stack()
        .map(|stack| stack.trim().to_string())
        .filter(|stack| !stack.is_empty());

    match (message, stack) {
        (Some(message), Some(stack)) if stack.contains(&message) => stack,
        (Some(message), Some(stack)) => format!("{message}\n{stack}"),
        (Some(message), None) => message,
        (None, Some(stack)) => stack,
        (None, None) => "JavaScript exception with no message or stack".to_string(),
    }
}
