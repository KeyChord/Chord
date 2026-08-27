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
    pub fn start(
        &self,
        task: &ChordActionTask,
        action: &HandlerChordAction,
    ) -> Result<HandlerChordActionTaskRun> {
        let handle = self.handle.clone();
        let handler_id = action.handler_id.clone();
        let event_args = action.event_args.clone();
        let num_times = task.num_times;
        let frontmost_id = task.event.application_id.clone();

        let join_handle = tauri::async_runtime::spawn(async move {
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
        });

        Ok(HandlerChordActionTaskRun { join_handle })
    }

    pub async fn end(&self, task_run: HandlerChordActionTaskRun) -> Result<()> {
        task_run.join_handle.await?
    }

    // TODO: implement deep aborting via AbortController
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
