use crate::app::SafeAppHandle;
use crate::quickjs::{format_js_error, with_js};
use anyhow::Result;
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
#[allow(unused_imports)]
use llrt_core::{Ctx, Function, Module, Object, Promise, Value};
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use typeshare::typeshare;
use crate::models::JavascriptChordAction;

#[typeshare(typescript(type = "any"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExportedFunctionInvocation {
    pub export_name: String,
    pub args: Vec<toml::Value>,
}

#[derive(Clone)]
pub struct JavascriptChordActionTaskRunner {
    handle: SafeAppHandle,
}

pub struct JavascriptChordActionTaskRun {
    join_handle: JoinHandle<Result<()>>
}

impl JavascriptChordActionTaskRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle }
    }
}

impl JavascriptChordActionTaskRunner {
    pub fn start(&self, action: JavascriptChordAction, num_times: u32) -> Result<JavascriptChordActionTaskRun> {
        let handle = self.handle.try_handle()?;
        let handle = handle.clone();
        let module_specifier = action.module_specifier.clone();

        let join_handle = tauri::async_runtime::spawn( async move {
            with_js(handle, move |ctx| {
                Box::pin(async move {
                    let args = action.args
                        .into_iter()
                        .map(|value| {
                            rquickjs_serde::to_value(ctx.clone(), value)
                                .or_throw_msg(&ctx, "Failed to convert TOML arguments")
                        })
                        .collect::<rquickjs::Result<Vec<_>>>()?;

                    for _ in 0..num_times {
                        call_js_export(ctx.clone(), &module_specifier, "default", args.clone())
                            .await
                            .map_err(|error| {
                                anyhow::anyhow!(
                                    "failed to execute JS default export:\n{}",
                                    format_js_error(&ctx, error)
                                )
                            })?;
                    }

                    Ok(())
                })
            }).await
        });

        Ok(JavascriptChordActionTaskRun {
            join_handle
        })
    }

    pub async fn end(&self, task_run: JavascriptChordActionTaskRun) -> Result<()> {
       task_run.join_handle.await?
    }

    // TODO: implement deep aborting via AbortController
    pub fn abort(&self, task_run: JavascriptChordActionTaskRun) -> Result<()> {
        task_run.join_handle.abort();
        Ok(())
    }
}

async fn call_js_export<'js>(
    ctx: Ctx<'js>,
    module_specifier: &str,
    export_name: &str,
    args: Vec<Value<'js>>,
) -> rquickjs::Result<()> {
    let module_promise = Module::import(&ctx, module_specifier.to_string())?;
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
    for value in args {
        args_builder.push_arg(value)?;
    }
    let mut result: Value<'js> = function.call_arg(args_builder)?;
    if let Some(promise) = result.as_promise().cloned() {
        result = promise.into_future::<Value<'js>>().await?;
    }
    if result.as_bool().is_some_and(|b| b == false) {
        log::error!("Function {} returned false:", export_name)
    }

    Ok(())
}
