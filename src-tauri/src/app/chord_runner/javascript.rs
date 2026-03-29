use std::pin::Pin;
use crate::app::SafeAppHandle;
use crate::quickjs::{format_js_error, with_js};
use anyhow::Result;
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
#[allow(unused_imports)]
use llrt_core::{Ctx, Function, Module, Object, Promise, Value};
use serde::Serialize;
use typeshare::typeshare;
use crate::app::chord_runner::{ChordActionTask, ChordActionTaskRun, ChordActionTaskRunner};
use crate::models::{ChordAction, ChordJavascriptAction};

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

struct JavascriptChordActionTaskRun {
    id: u32,
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl ChordActionTaskRun for JavascriptChordActionTaskRun {
    fn id(&self) -> u32 {
        self.id
    }
}

impl JavascriptChordActionTaskRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle }
    }
}

impl ChordActionTaskRunner for JavascriptChordActionTaskRunner {
    fn start(&self, task: ChordActionTask) -> Result<Option<Box<dyn ChordActionTaskRun>>> {
        let ChordAction::Javascript(action) = task.action else {
            return Ok(None);
        };

        let handle = self.handle.try_handle()?;
        let export_name = action.export_name.clone();

        let future = with_js(handle.clone(), move |ctx| {
            let args = convert_js_args(&ctx, action.args.clone());

            Box::pin(async move {
                for _ in 0..task.num_times {
                    call_js_export(ctx.clone(), "todo".into(), action.export_name, args?)
                        .await
                        .map_err(|error| {
                            anyhow::anyhow!(
                                "failed to execute JS export `{}`:\n{}",
                                export_name,
                                format_js_error(&ctx, error)
                            )
                        })?;
                }

                Ok(())
            })
        });

        let task_run: Option<Box<dyn ChordActionTaskRun>> = Some(
            Box::new(JavascriptChordActionTaskRun {
                id: 0,
                future: Box::pin(future) as Pin<Box<dyn Future<Output = ()> + Send>>,
            })
        );

        Ok(task_run)
    }

    /// No-op (most of the time). In the future, we'll likely return an object
    fn end(&self, task_run: JavascriptChordActionTaskRun) {}
    fn abort(&self, task_run: JavascriptChordActionTaskRun) {}
}

async fn call_js_export<'js>(
    ctx: Ctx<'js>,
    module_path: String,
    export_name: String,
    args: Vec<Value<'js>>,
) -> rquickjs::Result<()> {
    let module_promise = Module::import(&ctx, module_path.to_string())?;
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

fn convert_js_args<'js>(ctx: &Ctx<'js>, args: ChordJsArgs) -> rquickjs::Result<Vec<Value<'js>>> {
    match args {
        ChordJsArgs::Values(values) => values
            .into_iter()
            .map(|value| {
                rquickjs_serde::to_value(ctx.clone(), value)
                    .or_throw_msg(ctx, "Failed to convert TOML arguments")
            })
            .collect::<rquickjs::Result<Vec<_>>>(),

        ChordJsArgs::Eval(source) => {
            let value: Value<'js> = ctx.eval(source.as_str())?;
            if let Some(array) = value.as_array().cloned() {
                return (0..array.len())
                    .map(|index| {
                        array.get(index).or_throw_msg(
                            ctx,
                            &format!("Failed to read JS args `{}` at index {}", source, index),
                        )
                    })
                    .collect::<rquickjs::Result<Vec<_>>>();
            }

            Ok(vec![value])
        }
    }
}
