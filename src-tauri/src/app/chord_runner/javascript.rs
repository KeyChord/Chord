use crate::app::SafeAppHandle;
use crate::quickjs::with_js;
use anyhow::Result;
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
use llrt_core::{Ctx, Function, Module, Object, Promise, Value};
use serde::Serialize;
use typeshare::typeshare;

#[derive(Clone)]
pub struct ChordJavascriptRunner {
    handle: SafeAppHandle,
}

#[typeshare(typescript(type = "any"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum ChordJsArgs {
    #[typeshare(skip)]
    Values(Vec<toml::Value>),
    Eval(String),
}

#[typeshare(typescript(type = "any"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChordJsInvocation {
    pub export_name: String,
    pub args: ChordJsArgs,
}

impl ChordJavascriptRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle }
    }

    pub async fn execute_chord_javascript(
        &self,
        module_path: String,
        invocation: ChordJsInvocation,
        num_times: usize,
    ) -> Result<()> {
        let handle = self.handle.try_handle()?;
        with_js(handle.clone(), move |ctx| {
            Box::pin(call_js_export(ctx, module_path, invocation, num_times))
        })
        .await?;

        Ok(())
    }
}

async fn call_js_export<'js>(
    ctx: Ctx<'js>,
    module_path: String,
    invocation: ChordJsInvocation,
    num_times: usize,
) -> anyhow::Result<()> {
    for _ in 0..num_times {
        let export_name = invocation.export_name.clone();
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
        let args = convert_js_args(&ctx, invocation.args.clone())?;
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
            let array = value.as_array().cloned().or_throw_msg(
                ctx,
                &format!("JS args `{}` must evaluate to an array", source),
            )?;

            (0..array.len())
                .map(|index| {
                    array.get(index).or_throw_msg(
                        ctx,
                        &format!("Failed to read JS args `{}` at index {}", source, index),
                    )
                })
                .collect::<rquickjs::Result<Vec<_>>>()
        }
    }
}
