use crate::app::AppHandleExt;
use crate::app::chord_runner::shortcut::Shortcut;
use crate::app::desktop_app::{
    init_app_lifecycle, register_app_launch_handler, register_app_terminate_handler,
};
use crate::app::global_hotkey_store::GlobalHotkeyStoreEntry;
use crate::constants::GLOBAL_HOTKEYS_POOL;
use crate::quickjs::AppUserData;
use llrt_core::libs::utils::result::ResultExt;
use rquickjs::module::{Declarations, Exports, ModuleDef};
use rquickjs::prelude::Func;
use rquickjs::{Ctx, Function};
use std::collections::HashSet;

pub struct ChordModule;

fn on_app_launch<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<()> {
    register_app_launch_handler(ctx, bundle_id, callback)
}

fn on_app_terminate<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<Function<'js>> {
    register_app_terminate_handler(ctx, bundle_id, callback)
}

fn press(ctx: Ctx, key: String) -> rquickjs::Result<()> {
    let userdata = ctx.userdata::<AppUserData>().unwrap();
    let handle = &userdata.handle;
    let chord_runner = handle.chord_runner();
    let shortcut =
        Shortcut::parse(&key).or_throw_msg(&ctx, &format!("Invalid shortcut {}", key))?;
    chord_runner
        .shortcut
        .press_shortcut(shortcut, 1)
        .or_throw_msg(&ctx, "failed to press shortcut")?;
    Ok(())
}

fn release(ctx: Ctx, key: String) -> rquickjs::Result<()> {
    let userdata = ctx.userdata::<AppUserData>().unwrap();
    let handle = &userdata.handle;
    let chord_runner = handle.chord_runner();
    let shortcut =
        Shortcut::parse(&key).or_throw_msg(&ctx, &format!("Invalid shortcut {}", key))?;
    chord_runner
        .shortcut
        .release_shortcut(shortcut)
        .or_throw_msg(&ctx, "failed to release shortcut")?;
    Ok(())
}

fn tap(ctx: Ctx, key: String) -> rquickjs::Result<()> {
    let userdata = ctx.userdata::<AppUserData>().unwrap();
    let handle = &userdata.handle;
    let chord_runner = handle.chord_runner();
    let shortcut =
        Shortcut::parse(&key).or_throw_msg(&ctx, &format!("Invalid shortcut {}", key))?;
    chord_runner
        .shortcut
        .press_shortcut(shortcut.clone(), 1)
        .or_throw_msg(&ctx, "failed to press shortcut")?;
    chord_runner
        .shortcut
        .release_shortcut(shortcut.clone())
        .or_throw_msg(&ctx, "failed to press shortcut")?;
    Ok(())
}

fn get_global_hotkey(
    ctx: Ctx,
    bundle_id: String,
    hotkey_id: String,
) -> rquickjs::Result<Option<String>> {
    let userdata = ctx.userdata::<AppUserData>().unwrap();
    let handle = &userdata.handle;
    let global_hotkey_store = handle.app_global_hotkey_store();
    let shortcut = global_hotkey_store
        .entries()
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
    let userdata = ctx.userdata::<AppUserData>().unwrap();
    let handle = &userdata.handle;
    let global_hotkey_store = handle.app_global_hotkey_store();
    let all = global_hotkey_store.entries();

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
    let userdata = ctx.userdata::<AppUserData>().unwrap();
    let handle = &userdata.handle;
    let desktop_app_manager = handle.desktop_app_manager();
    desktop_app_manager
        .set_app_needs_relaunch(&bundle_id, needs_relaunch)
        .or_throw_msg(&ctx, "failed to set app relaunch flag")?;
    Ok(())
}

impl ModuleDef for ChordModule {
    fn declare(declare: &Declarations) -> rquickjs::Result<()> {
        declare.declare("press")?;
        declare.declare("release")?;
        declare.declare("tap")?;
        declare.declare("getGlobalHotkey")?;
        declare.declare("registerGlobalHotkey")?;
        declare.declare("setAppNeedsRelaunch")?;
        declare.declare("onAppLaunch")?;
        declare.declare("onAppTerminate")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let userdata = ctx.userdata::<AppUserData>().unwrap();
        let handle = &userdata.handle;
        init_app_lifecycle(handle.clone());

        exports.export("press", Func::from(press))?;
        exports.export("release", Func::from(release))?;
        exports.export("tap", Func::from(tap))?;
        exports.export("getGlobalHotkey", Func::from(get_global_hotkey))?;
        exports.export("registerGlobalHotkey", Func::from(register_global_hotkey))?;
        exports.export("setAppNeedsRelaunch", Func::from(set_app_needs_relaunch))?;
        exports.export("onAppLaunch", Func::from(on_app_launch))?;
        exports.export("onAppTerminate", Func::from(on_app_terminate))?;

        Ok(())
    }
}
