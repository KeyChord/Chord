use crate::quickjs::{format_js_error, with_js};
use anyhow::Result;
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
#[allow(unused_imports)]
use llrt_core::{Ctx, Function, Module, Object, Promise, Value};
use serde::Serialize;
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;
use typeshare::typeshare;
use crate::app::chord_runner::ChordActionTask;
use crate::models::HandlerChordAction;

#[derive(Clone)]
pub struct HandlerChordActionTaskRunner {
    handle: AppHandle,
}

#[derive(Debug)]
pub struct HandlerChordActionTaskRun {
    join_handle: JoinHandle<Result<()>>
}

impl HandlerChordActionTaskRunner {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl HandlerChordActionTaskRunner {
    pub fn start(&self, task: &ChordActionTask, action: &HandlerChordAction) -> Result<HandlerChordActionTaskRun> {
        let handle = self.handle.clone();
        let file = action.file.clone();
        let handler_args = action.handler_args.clone();
        let event_args = action.event_args.clone();
        let package_name = task.package_name.clone();
        let num_times = task.num_times;

        let join_handle = tauri::async_runtime::spawn( async move {
            with_js(handle, move |ctx| {
                Box::pin(async move {
                    let event_args = event_args
                        .into_iter()
                        .map(|value| {
                            rquickjs_serde::to_value(ctx.clone(), value)
                                .or_throw_msg(&ctx, "failed to convert event TOML arguments")
                        })
                        .collect::<rquickjs::Result<Vec<_>>>()?;

                    let handler_args = handler_args
                        .into_iter()
                        .map(|value| {
                            rquickjs_serde::to_value(ctx.clone(), value)
                                .or_throw_msg(&ctx, "failed to convert handler TOML arguments")
                        })
                        .collect::<rquickjs::Result<Vec<_>>>()?;

                    let module_specifier = format!("{}/js/{}", package_name, file);
                    let handler = get_default_export(ctx.clone(), &module_specifier, handler_args.clone())
                        .await
                        .map_err(|error| {
                            anyhow::anyhow!(
                                "failed to execute JS default export:\n{}",
                                format_js_error(&ctx, error)
                            )
                        })?;

                    for _ in 0..num_times {
                        let mut args_builder = Args::new(ctx.clone(), event_args.len());
                        for value in event_args.clone() {
                            args_builder.push_arg(value)?;
                        }
                        let mut result: Value = handler.call_arg(args_builder)?;
                        if let Some(promise) = result.as_promise().cloned() {
                            result = promise.into_future::<Value>().await?;
                        }

                        log::debug!("handler task result: {:?}", result);
                    }

                    Ok(())
                })
            }).await
        });

        Ok(HandlerChordActionTaskRun {
            join_handle
        })
    }

    pub async fn end(&self, task_run: HandlerChordActionTaskRun) -> Result<()> {
       task_run.join_handle.await?
    }

    // TODO: implement deep aborting via AbortController
    pub fn abort(&self, task_run: HandlerChordActionTaskRun) -> Result<()> {
        task_run.join_handle.abort();
        Ok(())
    }
}

async fn get_default_export<'js>(
    ctx: Ctx<'js>,
    module_specifier: &str,
    args: Vec<Value<'js>>,
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
