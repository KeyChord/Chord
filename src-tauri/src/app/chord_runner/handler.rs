use crate::quickjs::{format_js_error, with_js};
use anyhow::{Context, Result};
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
#[allow(unused_imports)]
use llrt_core::{Ctx, Function, Module, Object, Promise, Value};

use crate::app::AppHandleExt;
use crate::app::chord_runner::ChordActionTask;
use crate::models::HandlerChordAction;
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;

#[derive(Clone)]
pub struct HandlerChordActionTaskRunner {
    handle: AppHandle,
}

#[derive(Debug)]
pub struct HandlerChordActionTaskRun {
    join_handle: JoinHandle<Result<()>>,
}

impl HandlerChordActionTaskRunner {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl HandlerChordActionTaskRunner {
    pub fn start(
        &self,
        task: &ChordActionTask,
        action: &HandlerChordAction,
    ) -> Result<HandlerChordActionTaskRun> {
        let handle = self.handle.clone();
        let file = action.file.clone();
        let build_args = action.build_args.clone();
        let event_args = action.event_args.clone();
        let package_name = task.package_name.clone();
        let num_times = task.num_times;

        let chord_pm = self.handle.chord_package_manager();
        let package = chord_pm
            .get_package_by_name(&package_name)
            .context("could not get package")?;
        let initiator_chords_file = package
            .raw_chords_files
            .get(&task.initiator_file_pathslug)
            .context("could not get chord file")?;
        let context_chords_file = serde_json::to_value(initiator_chords_file.clone())?;

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

                        let build_args = build_args
                            .into_iter()
                            .map(|value| {
                                rquickjs_serde::to_value(ctx.clone(), value)
                                    .or_throw_msg(&ctx, "failed to convert handler TOML arguments")
                            })
                            .collect::<rquickjs::Result<Vec<_>>>()?;

                        let module_specifier = format!("{}/js/{}", package_name, file);
                        log::debug!("retrieving default export of {}", module_specifier);
                        let build_handler_fn =
                            get_default_export(ctx.clone(), &module_specifier).await?;

                        let mut args = Args::new(ctx.clone(), build_args.len());
                        let build_context = Object::new(ctx.clone())?;
                        build_context.set(
                            "chordsFile",
                            rquickjs_serde::to_value(ctx.clone(), context_chords_file)
                                .or_throw_msg(&ctx, "failed to parse chords file")?,
                        )?;
                        args.this(build_context)?;
                        for value in build_args.clone() {
                            args.push_arg(value)?;
                        }
                        log::debug!("calling build_handler with args {:?}", build_args);
                        let mut handler: Value = build_handler_fn.call_arg(args)?;
                        if let Some(promise) = handler.as_promise().cloned() {
                            handler = promise.into_future::<Value>().await?;
                        }
                        let handler_fn = handler.as_function().or_throw_msg(
                            &ctx,
                            "the default export function must return a function",
                        )?;

                        for _ in 0..num_times {
                            let mut args = Args::new(ctx.clone(), event_args.len());
                            for value in event_args.clone() {
                                args.push_arg(value)?;
                            }
                            let handler_context = Object::new(ctx.clone())?;
                            // TODO
                            handler_context.set("focusedAppId", "")?;
                            args.this(handler_context)?;

                            log::debug!("calling handler with args {:?}", event_args);
                            let mut result: Value = handler_fn.call_arg(args)?;
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

