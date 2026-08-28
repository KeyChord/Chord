use crate::app::AppHandleExt;
use crate::app::chord_runner::ChordActionTask;
use crate::models::HandlerChordAction;
use crate::quickjs::{format_js_error, with_js};
use crate::state::{FrontmostObservable, FrontmostState};
use anyhow::{Context, Result};
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
use llrt_core::{Ctx, Function, Module, Object, Promise, Value};
use nject::injectable;
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;

#[injectable]
#[derive(Clone)]
pub struct HandlerChordActionTaskRunner {
    handle: AppHandle,
}

#[derive(Debug)]
pub struct HandlerChordActionTaskRun {
    join_handle: JoinHandle<Result<()>>,
}

impl HandlerChordActionTaskRunner {
    /// Runs the handler on whichever JS engine this process uses (see `js_engine`).
    pub fn start(
        &self,
        task: &ChordActionTask,
        action: &HandlerChordAction,
    ) -> Result<HandlerChordActionTaskRun> {
        #[cfg(feature = "bun")]
        let use_bun = crate::js_engine::select(&self.handle) == crate::js_engine::JsEngine::Bun;
        #[cfg(not(feature = "bun"))]
        let use_bun = false;

        #[cfg(feature = "bun")]
        let join_handle = if use_bun {
            self.start_js_bun(task, action)
        } else {
            self.start_js(task, action)
        };
        #[cfg(not(feature = "bun"))]
        let join_handle = {
            let _ = use_bun;
            self.start_js(task, action)
        };
        Ok(HandlerChordActionTaskRun { join_handle })
    }

    fn start_js(
        &self,
        task: &ChordActionTask,
        action: &HandlerChordAction,
    ) -> JoinHandle<Result<()>> {
        let handle = self.handle.clone();
        let handler_id = action.handler_id.clone();
        let event_args = action.event_args.clone();
        let num_times = task.num_times;
        let frontmost_id = task.event.application_id.clone();

        tauri::async_runtime::spawn(async move {
            with_js(handle, move |ctx| {
                Box::pin(async move {
                    async {
                        let event_args = event_args
                            .into_iter()
                            .map(|value| {
                                rquickjs_serde::to_value(ctx.clone(), value)
                                    .or_throw_msg(&ctx, "failed to convert event TOML arguments")
                            })
                            .collect::<rquickjs::Result<Vec<_>>>()?;

                        let globals = ctx.globals();
                        let registry: Object = globals
                            .get("__RUST_HANDLER_REGISTRY")
                            .or_throw_msg(&ctx, "Global handler registry not found")?;

                        let handler_function: Function = registry.get(&handler_id).or_throw_msg(
                            &ctx,
                            &format!("Handler ID '{}' not found in registry", handler_id),
                        )?;

                        for _ in 0..num_times {
                            let mut args = Args::new(ctx.clone(), event_args.len());
                            for value in event_args.clone() {
                                args.push_arg(value)?;
                            }
                            let handler_context = Object::new(ctx.clone())?;
                            handler_context.set("focusedAppId", frontmost_id.clone())?;
                            args.this(handler_context)?;

                            log::debug!("calling handler with args {:?}", event_args);
                            let mut result: Value = handler_function.call_arg(args)?;
                            if let Some(promise) = result.as_promise().cloned() {
                                result = promise.into_future::<Value>().await?;
                            }

                            log::debug!("handler task result: {:?}", result);
                        }

                        Ok::<(), rquickjs::Error>(())
                    }
                    .await
                    .map_err(|e| anyhow::Error::msg(format_js_error(&ctx, e)))
                })
            })
            .await
        })
    }

    /// Bun engine counterpart of [`Self::start_js`].
    #[cfg(feature = "bun")]
    fn start_js_bun(
        &self,
        task: &ChordActionTask,
        action: &HandlerChordAction,
    ) -> JoinHandle<Result<()>> {
        bun_impl::start(
            self.handle.clone(),
            action.handler_id.clone(),
            action.event_args.clone(),
            task.num_times,
            task.event.application_id.clone(),
        )
    }

    pub async fn end(&self, task_run: HandlerChordActionTaskRun) -> Result<()> {
        task_run.join_handle.await?
    }

    // TODO: implement deep aborting of JS via AbortController
    #[allow(dead_code)]
    pub fn abort(&self, task_run: HandlerChordActionTaskRun) -> Result<()> {
        task_run.join_handle.abort();
        Ok(())
    }
}

async fn get_default_export<'js>(
    ctx: Ctx<'js>,
    module_specifier: &str,
) -> rquickjs::Result<Function<'js>> {
    let module_promise = Module::import(&ctx, module_specifier.to_string())?;
    let module = module_promise.into_future::<Object>().await?;
    let mut export: Value<'js> = module.get("default")?;
    if let Some(promise) = export.as_promise().cloned() {
        export = promise.into_future::<Value<'js>>().await?;
    }
    let function = export.as_function().cloned().or_throw_msg(
        &ctx,
        &format!(
            "JS default export did not resolve to a function: {:?}",
            export
        ),
    )?;
    Ok(function)
}

/// The Bun engine's handler invocation, kept in its own module so rbun's and
/// LLRT's `ResultExt` never share a scope.
#[cfg(feature = "bun")]
mod bun_impl {
    use crate::bun_js::{format_js_error, with_js};
    use anyhow::Result;
    use rbun::prelude::{Args, Object, ResultExt, Value};
    use rbun::Function;
    use tauri::AppHandle;
    use tauri::async_runtime::JoinHandle;

    pub fn start(
        handle: AppHandle,
        handler_id: String,
        event_args: Vec<toml::Value>,
        num_times: u32,
        frontmost_id: Option<String>,
    ) -> JoinHandle<Result<()>> {
        tauri::async_runtime::spawn(async move {
            with_js(handle, move |ctx| {
                Box::pin(async move {
                    async {
                        let event_args = event_args
                            .into_iter()
                            .map(|value| {
                                rbun::serde::to_value(ctx.clone(), value)
                                    .or_throw_msg(&ctx, "failed to convert event TOML arguments")
                            })
                            .collect::<rbun::Result<Vec<_>>>()?;

                        let globals = ctx.globals();
                        let registry: Object = globals
                            .get("__RUST_HANDLER_REGISTRY")
                            .or_throw_msg(&ctx, "Global handler registry not found")?;

                        let handler_function: Function = registry.get(&handler_id).or_throw_msg(
                            &ctx,
                            &format!("Handler ID '{}' not found in registry", handler_id),
                        )?;

                        for _ in 0..num_times {
                            let mut args = Args::new(ctx.clone(), event_args.len());
                            for value in event_args.clone() {
                                args.push_arg(value)?;
                            }
                            let handler_context = Object::new(ctx.clone())?;
                            handler_context.set("focusedAppId", frontmost_id.clone())?;
                            args.this(handler_context)?;

                            log::debug!("calling handler with args {:?} (bun)", event_args);
                            let mut result: Value = handler_function.call_arg(args)?;
                            if let Some(promise) = result.as_promise().cloned() {
                                result = promise.into_future::<Value>().await?;
                            }

                            log::debug!("handler task result: {:?}", result);
                        }

                        Ok::<(), rbun::Error>(())
                    }
                    .await
                    .map_err(|e| anyhow::Error::msg(format_js_error(&ctx, e)))
                })
            })
            .await
        })
    }
}
