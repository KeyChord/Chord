use crate::app::chord_runner::ChordActionTask;
#[cfg(all(test, target_os = "macos"))]
use crate::bun_js::format_js_error;
use crate::bun_js::package_worker;
use crate::models::HandlerChordAction;
use anyhow::Result;
use nject::injectable;
#[cfg(target_os = "macos")]
use rbun::prelude::{Args, Object, OptionExt, ResultExt, Value};
#[cfg(target_os = "macos")]
use rbun::{Array, Ctx, Function};
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
    handler_id: String,
}

impl HandlerChordActionTaskRunner {
    pub fn start(
        &self,
        task: &ChordActionTask,
        action: &HandlerChordAction,
    ) -> Result<HandlerChordActionTaskRun> {
        Ok(HandlerChordActionTaskRun {
            join_handle: self.start_js(task, action),
            handler_id: action.handler_id.clone(),
        })
    }

    fn start_js(
        &self,
        task: &ChordActionTask,
        action: &HandlerChordAction,
    ) -> JoinHandle<Result<()>> {
        let action = action.clone();
        let num_times = task.num_times;
        let frontmost_id = task.event.application_id.clone();

        tauri::async_runtime::spawn(async move {
            package_worker::invoke(action, frontmost_id, num_times).await
        })
    }

    pub async fn end(&self, task_run: &mut HandlerChordActionTaskRun) -> Result<()> {
        (&mut task_run.join_handle).await?
    }

    #[allow(dead_code)]
    pub fn abort(&self, task_run: HandlerChordActionTaskRun) -> Result<()> {
        package_worker::abort(&task_run.handler_id);
        task_run.join_handle.abort();
        Ok(())
    }
}

#[cfg(target_os = "macos")]
const SEQUENCE_VALUES: &str = "__RUST_CHORD_SEQUENCE_VALUES";

#[cfg(target_os = "macos")]
fn sequence_registry<'js>(ctx: Ctx<'js>) -> rbun::Result<Object<'js>> {
    if let Some(registry) = ctx.globals().get::<_, Option<Object>>(SEQUENCE_VALUES)? {
        return Ok(registry);
    }
    let registry = Object::new(ctx.clone())?;
    registry.set_prototype(None)?;
    ctx.globals().set(SEQUENCE_VALUES, registry.clone())?;
    Ok(registry)
}

#[cfg(target_os = "macos")]
fn sequence_values<'js>(ctx: Ctx<'js>, id: &str) -> rbun::Result<Object<'js>> {
    let registry = sequence_registry(ctx.clone())?;
    if let Some(values) = registry.get::<_, Option<Object>>(id)? {
        return Ok(values);
    }
    let values = Object::new(ctx)?;
    values.set_prototype(None)?;
    registry.set(id, values.clone())?;
    Ok(values)
}

pub async fn release_chord_sequences(handle: AppHandle, ids: Vec<String>) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let _ = handle;
    package_worker::release_sequences(ids).await;
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn release_sequence_values(ctx: Ctx<'_>, ids: Vec<String>) -> rbun::Result<()> {
    if let Some(registry) = ctx.globals().get::<_, Option<Object>>(SEQUENCE_VALUES)? {
        for id in ids {
            registry.remove(id)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn resolve_event_argument<'js>(
    ctx: Ctx<'js>,
    value: &toml::Value,
    scope: Option<&Object<'js>>,
) -> rbun::Result<Value<'js>> {
    if let Some(identifier) = value
        .as_str()
        .and_then(|s| s.strip_prefix('$'))
        .filter(|s| crate::models::is_event_identifier(s))
    {
        let Some(scope) = scope else {
            return Err(rbun::Exception::throw_reference(
                &ctx,
                &format!("${identifier} has not been assigned for this chord's matching interval"),
            ));
        };
        if !scope.contains_key(identifier)? {
            return Err(rbun::Exception::throw_reference(
                &ctx,
                &format!("${identifier} has not been assigned for this chord's matching interval"),
            ));
        }
        return scope.get(identifier);
    }
    match value {
        toml::Value::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (i, value) in values.iter().enumerate() {
                array.set(i, resolve_event_argument(ctx.clone(), value, scope)?)?;
            }
            Ok(array.into_value())
        }
        toml::Value::Table(values) => {
            let object = Object::new(ctx.clone())?;
            for (key, value) in values {
                object.set(
                    key.as_str(),
                    resolve_event_argument(ctx.clone(), value, scope)?,
                )?;
            }
            Ok(object.into_value())
        }
        _ => rbun::serde::to_value(ctx, value.clone()),
    }
}

#[cfg(target_os = "macos")]
pub(crate) async fn invoke_js_handler<'js>(
    ctx: Ctx<'js>,
    action: &HandlerChordAction,
    frontmost_id: Option<String>,
    num_times: u32,
) -> rbun::Result<()> {
    let registry: Object = ctx
        .globals()
        .get("__RUST_HANDLER_REGISTRY")
        .or_throw_msg(&ctx, "Global handler registry not found")?;
    let handler: Function = registry.get(&action.handler_id).or_throw_msg(
        &ctx,
        &format!("Handler ID '{}' not found in registry", action.handler_id),
    )?;
    let scope = action
        .sequence_id
        .as_deref()
        .map(|id| sequence_values(ctx.clone(), id))
        .transpose()?;
    for _ in 0..num_times {
        let mut args = Args::new(ctx.clone(), action.event_args.len());
        for arg in &action.event_args {
            args.push_arg(resolve_event_argument(ctx.clone(), arg, scope.as_ref())?)?;
        }
        let handler_context = Object::new(ctx.clone())?;
        handler_context.set("focusedAppId", frontmost_id.clone())?;
        // Selected once for the whole run, so repetitions never follow focus elsewhere.
        match action.focused_window_id {
            Some(id) => handler_context.set("focusedWindowId", id)?,
            None => handler_context.set("focusedWindowId", Value::new_null(ctx.clone()))?,
        }
        handler_context.set("input", action.input.clone())?;
        if let Some(event) = &action.keyboard_event {
            let event_value = rbun::serde::to_value(ctx.clone(), event)?;
            let create_event: Function = ctx.eval(r#"(function(data) {
                const event = new Event(data.type);
                for (const [key, value] of Object.entries(data)) {
                    if (key !== 'type') Object.defineProperty(event, key, { value, enumerable: true });
                }
                Object.defineProperties(event, {
                    view: { value: null },
                    detail: { value: 0 },
                    getModifierState: { value: function(key) {
                        return ({ Alt: this.altKey, Control: this.ctrlKey, Meta: this.metaKey, Shift: this.shiftKey })[key] === true;
                    } }
                });
                return event;
            })"#)?;
            let event_object: Object = create_event.call((event_value,))?;
            handler_context.set("event", event_object)?;
        } else {
            handler_context.set("event", Value::new_null(ctx.clone()))?;
        }
        args.this(handler_context)?;
        let mut result: Value = handler.call_arg(args)?;
        if let Some(promise) = result.as_promise().cloned() {
            result = promise.into_future::<Value>().await?;
        }
        if let Some(identifier) = &action.assign_to {
            let scope = scope.as_ref().or_throw_msg(
                &ctx,
                "Return-value assignment requires a chord matching interval",
            )?;
            scope.set(identifier.as_str(), result)?;
        }
    }
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::models::{ChordKeyboardEvent, Key, KeyEvent};
    use keycode::KeyMappingCode;

    fn action(
        handler: &str,
        scope: &str,
        assignment: Option<&str>,
        args: Vec<toml::Value>,
    ) -> HandlerChordAction {
        HandlerChordAction {
            handler_id: handler.into(),
            event_args: args,
            input: "j,12".into(),
            sequence_id: Some(scope.into()),
            assign_to: assignment.map(str::to_string),
            keyboard_event: Some(ChordKeyboardEvent::new(
                &KeyEvent::Press(Key(KeyMappingCode::Backspace)),
                &[Key(KeyMappingCode::ShiftLeft)],
                false,
            )),
            focused_window_id: None,
        }
    }

    #[tokio::test]
    async fn return_values_preserve_identity_and_promises_and_keyboard_event_context() {
        crate::bun_js::with_test_js(|ctx| Box::pin(async move {
            let result = async {
                ctx.eval::<(), _>(r#"
globalThis.__RUST_HANDLER_REGISTRY = {
    seqPrepare: async function() {
        await Promise.resolve();
        const state = { count: 0, fn: () => 7 };
        state.self = state;
        globalThis.seqOriginal = state;
        return state;
    },
    seqDown: function(state, nested) {
        if (state !== globalThis.seqOriginal || state.self !== state || nested[0] !== state) throw new Error('identity lost');
        if (!(this.event instanceof Event) || this.event.type !== 'keydown' || this.event.key !== 'Backspace' || this.event.code !== 'Backspace' || !this.event.getModifierState('Shift') || this.event.getModifierState('Control')) throw new Error('incorrect keyboard event');
        if (this.event.target !== null || this.event.cancelable !== false) throw new Error('incorrect Event shape');
        if (this.focusedAppId !== 'dev.fixture') throw new Error('missing app context');
        if (this.input !== 'j,12') throw new Error('missing chord input');
        state.count++;
        return state.fn() + state.count;
    },
    seqUp: function(value) { globalThis.seqObserved = value; },
    seqUndefined: function() { return undefined; }
};
"#)?;
                let init = action("seqPrepare", "return-value-test", Some("state"), vec![]);
                invoke_js_handler(ctx.clone(), &init, Some("dev.fixture".into()), 1).await?;
                let down = action("seqDown", "return-value-test", Some("result"), vec!["$state".into(), toml::Value::Array(vec!["$state".into()])]);
                invoke_js_handler(ctx.clone(), &down, Some("dev.fixture".into()), 1).await?;
                invoke_js_handler(ctx.clone(), &action("seqUp", "return-value-test", None, vec!["$result".into()]), None, 1).await?;
                assert_eq!(ctx.eval::<i32, _>("globalThis.seqObserved")?, 8);
                // Undefined is still an assigned value, and identifier names cannot alter prototypes.
                invoke_js_handler(ctx.clone(), &action("seqUndefined", "return-value-test", Some("__proto__"), vec![]), None, 1).await?;
                invoke_js_handler(ctx.clone(), &action("seqUp", "return-value-test", None, vec!["$__proto__".into()]), None, 1).await?;
                assert!(ctx.eval::<bool, _>("globalThis.seqObserved === undefined")?);
                let error = invoke_js_handler(ctx.clone(), &action("seqUp", "other-matching-interval", None, vec!["$state".into()]), None, 1).await.unwrap_err();
                assert!(format_js_error(&ctx, error).contains("$state has not been assigned"));
                sequence_registry(ctx.clone())?.remove("return-value-test")?;
                assert!(!sequence_values(ctx.clone(), "return-value-test")?.contains_key("state")?);
                Ok::<(), rbun::Error>(())
            }.await;
            result.map_err(|error| anyhow::anyhow!(format_js_error(&ctx, error)))
        })).await.unwrap();
    }
}
