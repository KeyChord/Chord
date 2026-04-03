use crate::app::AppHandleExt;
use crate::app::desktop_app::{
    init_app_lifecycle, register_app_launch_handler, register_app_terminate_handler,
};
use crate::app::global_hotkey_store::GlobalHotkeyStoreEntry;
use crate::constants::GLOBAL_HOTKEYS_POOL;
use crate::models::{ShortcutChordAction, SimulatedShortcut};
use crate::quickjs::AppUserData;
use llrt_core::libs::json::stringify::json_stringify;
use llrt_core::libs::utils::result::ResultExt;
#[cfg(target_os = "macos")]
use osakit::{Language as OsaLanguage, Script as OsaScript};
use rquickjs::Class;
use rquickjs::class::{JsClass, Trace, Tracer};
use rquickjs::module::{Declarations, Exports, ModuleDef};
use rquickjs::prelude::{Func, Rest, This};
use rquickjs::{Array, Ctx, Exception, Function, JsLifetime, Value};
use std::collections::HashSet;
use rquickjs::function::Async;

pub struct ChordModule;

enum ApplescriptLanguage {
    AppleScript,
    JavaScript,
}

#[rquickjs::class]
struct Applescript {
    #[cfg(target_os = "macos")]
    script: OsaScript,
    #[cfg(not(target_os = "macos"))]
    source: String,
}

unsafe impl<'js> JsLifetime<'js> for Applescript {
    type Changed<'to> = Applescript;
}

impl<'js> Trace<'js> for Applescript {
    fn trace(&self, _tracer: Tracer<'_, 'js>) {}
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Applescript {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<Self> {
        let mut args_iter = args.0.into_iter();
        let Some(first_arg) = args_iter.next() else {
            return Err(Exception::throw_message(
                &ctx,
                "Applescript constructor expects a source string or function",
            ));
        };

        if let Some(source) = first_arg.as_string() {
            if args_iter.next().is_some() {
                return Err(Exception::throw_message(
                    &ctx,
                    "Applescript only accepts extra arguments when constructed from a function",
                ));
            }

            return Ok(Self::from_source(
                ApplescriptLanguage::AppleScript,
                source.to_string()?,
            ));
        }

        if let Some(function) = first_arg.as_function() {
            let function_source = function_source(&ctx, &function)?;
            let wrapped_source = wrap_jxa_function(&ctx, function_source, args_iter.collect())?;
            return Ok(Self::from_source(
                ApplescriptLanguage::JavaScript,
                wrapped_source,
            ));
        }

        Err(Exception::throw_message(
            &ctx,
            "Applescript constructor expects a source string or function",
        ))
    }

    fn compile<'js>(&mut self, ctx: Ctx<'js>) -> rquickjs::Result<()> {
        #[cfg(target_os = "macos")]
        {
            self.script.compile().map_err(|error| {
                Exception::throw_message(&ctx, &format!("failed to compile AppleScript: {error}"))
            })?;
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = &self.source;
            Err(Exception::throw_message(
                &ctx,
                "AppleScript is only supported on macOS",
            ))
        }
    }

    fn execute<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        #[cfg(target_os = "macos")]
        {
            let value = self.script.execute().map_err(|error| {
                Exception::throw_message(&ctx, &format!("failed to execute AppleScript: {error}"))
            })?;

            rquickjs_serde::to_value(ctx.clone(), value)
                .or_throw_msg(&ctx, "failed to convert AppleScript result")
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = &self.source;
            Err(Exception::throw_message(
                &ctx,
                "AppleScript is only supported on macOS",
            ))
        }
    }
}

impl Applescript {
    fn from_source(language: ApplescriptLanguage, source: String) -> Self {
        Self {
            #[cfg(target_os = "macos")]
            script: OsaScript::new_from_source(
                match language {
                    ApplescriptLanguage::AppleScript => OsaLanguage::AppleScript,
                    ApplescriptLanguage::JavaScript => OsaLanguage::JavaScript,
                },
                &source,
            ),
            #[cfg(not(target_os = "macos"))]
            source,
        }
    }
}

fn function_source<'js>(ctx: &Ctx<'js>, function: &Function<'js>) -> rquickjs::Result<String> {
    let to_string: Function<'js> = function
        .get("toString")
        .or_throw_msg(ctx, "failed to read function.toString")?;

    to_string
        .call((This(function.clone()),))
        .or_throw_msg(ctx, "failed to stringify function source")
}

fn wrap_jxa_function<'js>(
    ctx: &Ctx<'js>,
    function_source: String,
    args: Vec<Value<'js>>,
) -> rquickjs::Result<String> {
    let serialized_args = serialize_jxa_args(ctx, args)?;

    Ok(format!(
        "var __chordArgs = {serialized_args};\nvar __chordFn = ({function_source});\noutput = __chordFn.apply(null, __chordArgs);"
    ))
}

fn serialize_jxa_args<'js>(ctx: &Ctx<'js>, args: Vec<Value<'js>>) -> rquickjs::Result<String> {
    let array = Array::new(ctx.clone())?;
    for (index, value) in args.into_iter().enumerate() {
        array.set(index, value)?;
    }

    json_stringify(ctx, array.into_value())?
        .ok_or_else(|| Exception::throw_message(ctx, "failed to serialize JXA arguments"))
}

fn app_handle(ctx: &Ctx<'_>) -> rquickjs::Result<tauri::AppHandle> {
    let userdata = ctx
        .userdata::<AppUserData>()
        .ok_or_else(|| Exception::throw_message(ctx, "missing app context"))?;

    userdata
        .handle
        .clone()
        .ok_or_else(|| Exception::throw_message(ctx, "`chord` module is unavailable in CLI mode"))
}

fn on_app_launch<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<()> {
    let _ = app_handle(&ctx)?;
    register_app_launch_handler(ctx, bundle_id, callback)
}

fn on_app_terminate<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<Function<'js>> {
    let _ = app_handle(&ctx)?;
    register_app_terminate_handler(ctx, bundle_id, callback)
}

fn press(ctx: Ctx, key: String) -> rquickjs::Result<()> {
    let handle = app_handle(&ctx)?;
    let runner = handle.chord_action_task_runner();
    let simulated_shortcut = key
        .parse::<SimulatedShortcut>()
        .or_throw_msg(&ctx, &format!("Invalid shortcut {}", key))?;
    runner
        .shortcut
        .simulate_shortcut_actions(
            runner.shortcut.get_start_simulated_shortcut_actions(
                &ShortcutChordAction { simulated_shortcut },
                1,
            ),
        )
        .or_throw_msg(&ctx, "failed to press shortcut")?;
    Ok(())
}

fn release(ctx: Ctx, key: String) -> rquickjs::Result<()> {
    let handle = app_handle(&ctx)?;
    let runner = handle.chord_action_task_runner();
    let simulated_shortcut = key
        .parse::<SimulatedShortcut>()
        .or_throw_msg(&ctx, &format!("Invalid shortcut {}", key))?;
    runner
        .shortcut
        .simulate_shortcut_actions(
            runner
                .shortcut
                .get_end_simulated_shortcut_actions(&ShortcutChordAction { simulated_shortcut }),
        )
        .or_throw_msg(&ctx, "failed to release shortcut")?;
    Ok(())
}

fn tap(ctx: Ctx, key: String) -> rquickjs::Result<()> {
    press(ctx.clone(), key.clone())?;
    release(ctx.clone(), key)?;
    Ok(())
}

fn get_global_hotkey(
    ctx: Ctx,
    bundle_id: String,
    hotkey_id: String,
) -> rquickjs::Result<Option<String>> {
    let handle = app_handle(&ctx)?;
    let global_hotkey_store = handle.app_global_hotkey_store();
    let shortcut = global_hotkey_store
        .entries()
        .or_throw_msg(&ctx, "bad entries")?
        .into_iter()
        .find_map(|(shortcut, entry)| {
            (entry.bundle_id == bundle_id && entry.hotkey_id == hotkey_id).then_some(shortcut)
        });

    Ok(shortcut)
}

fn register_global_hotkey(
    ctx: Ctx,
    bundle_id: String,
    hotkey_id: String,
) -> rquickjs::Result<Option<String>> {
    let handle = app_handle(&ctx)?;
    let global_hotkey_store = handle.app_global_hotkey_store();
    let all = global_hotkey_store
        .entries()
        .or_throw_msg(&ctx, "bad store")?;

    // idempotent: if this hotkey is already registered, return the existing shortcut
    if let Some(existing) = all.iter().find_map(|(shortcut, entry)| {
        (entry.bundle_id == bundle_id && entry.hotkey_id == hotkey_id).then_some(shortcut.clone())
    }) {
        return Ok(Some(existing));
    }

    let used: HashSet<String> = all.into_keys().collect();

    let Some(next) = GLOBAL_HOTKEYS_POOL
        .iter()
        .find(|shortcut| !used.contains(&shortcut.serialize()))
        .cloned()
    else {
        return Ok(None);
    };

    let shortcut = next.serialize();

    global_hotkey_store
        .set(
            &shortcut,
            GlobalHotkeyStoreEntry {
                bundle_id,
                hotkey_id,
            },
        )
        .or_throw_msg(&ctx, "failed to save global hotkey")?;

    Ok(Some(shortcut))
}

fn set_app_needs_relaunch(
    ctx: Ctx,
    bundle_id: String,
    needs_relaunch: bool,
) -> rquickjs::Result<()> {
    let handle = app_handle(&ctx)?;
    let desktop_app_manager = handle.desktop_app_manager();
    desktop_app_manager
        .set_app_needs_relaunch(&bundle_id, needs_relaunch)
        .or_throw_msg(&ctx, "failed to set app relaunch flag")?;
    Ok(())
}

async fn run_sudo_command<'js>(
    ctx: Ctx<'js>,
    program: String,
    args: Array<'js>,
) -> rquickjs::Result<()> {
    let mut rust_args: Vec<String> = Vec::new();
    for item in args.into_iter() {
        let arg_string: String = item?.get()?;
        rust_args.push(arg_string);
    }

    let thread_result = tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = std::process::Command::new(program);
        for arg in rust_args {
            cmd.arg(arg);
        }

        // Execute and return a standard `std::io::Result` from the closure
        if elevated_command::Command::is_elevated() {
            cmd.output().map_err(|e| e.to_string())
        } else {
            elevated_command::Command::new(cmd).output().map_err(|e| e.to_string())
        }
    })
        .await;

    // 3. Handle the thread result back on the JS context thread.
    match thread_result {
        // Thread succeeded, command executed successfully
        Ok(Ok(_output)) => Ok(()),

        // Command failed to run (e.g., UAC cancelled, binary not found)
        Ok(Err(e)) => {
            // Throw a JS exception using the context
            Err(Exception::throw_message(&ctx, &format!("failed to run command: {}", e)))
        }

        // Tokio `spawn_blocking` failed (the background thread panicked)
        Err(e) => {
            Err(Exception::throw_message(&ctx, &format!("background thread failed: {}", e)))
        }
    }
}
impl ModuleDef for ChordModule {
    fn declare(declare: &Declarations) -> rquickjs::Result<()> {
        declare.declare("Applescript")?;
        declare.declare("press")?;
        declare.declare("release")?;
        declare.declare("tap")?;
        declare.declare("getGlobalHotkey")?;
        declare.declare("registerGlobalHotkey")?;
        declare.declare("setAppNeedsRelaunch")?;
        declare.declare("onAppLaunch")?;
        declare.declare("onAppTerminate")?;
        declare.declare("runSudoCommand")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        if let Ok(handle) = app_handle(ctx) {
            init_app_lifecycle(handle);
        }

        Class::<Applescript>::define(&ctx.globals())?;
        let applescript_ctor: Value<'js> = ctx.globals().get(Applescript::NAME)?;

        exports.export("Applescript", applescript_ctor)?;
        exports.export("press", Func::from(press))?;
        exports.export("release", Func::from(release))?;
        exports.export("tap", Func::from(tap))?;
        exports.export("getGlobalHotkey", Func::from(get_global_hotkey))?;
        exports.export("registerGlobalHotkey", Func::from(register_global_hotkey))?;
        exports.export("setAppNeedsRelaunch", Func::from(set_app_needs_relaunch))?;
        exports.export("onAppLaunch", Func::from(on_app_launch))?;
        exports.export("onAppTerminate", Func::from(on_app_terminate))?;
        exports.export("runSudoCommand", Func::from(Async(run_sudo_command)))?;

        Ok(())
    }
}
