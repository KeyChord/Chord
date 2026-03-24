use crate::chords::{Chord, ChordPackage, ChordRegistry, GLOBAL_CHORD_RUNTIME_ID};
use crate::feature::app_handle_ext::AppHandleExt;
use crate::js::{format_js_error, reset_js, with_js};
use crate::observables::{ChorderObservable, FrontmostObservable, Observable};
use crate::{
    input::KeyEventState,
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::Result;
use base64::Engine;
use device_query::DeviceState;
use keycode::KeyMappingCode::*;
use llrt_core::libs::utils::result::ResultExt;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSRunningApplication, NSWorkspace,
    NSWorkspaceLaunchOptions,
};
use objc2_foundation::{NSDictionary, NSSize, NSString};
use parking_lot::RwLock;
use rquickjs::{Ctx, Module};
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};

const APPS_NEEDING_RELAUNCH_CHANGED_EVENT: &str = "apps-needing-relaunch-changed";

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct AppNeedsRelaunchInfo {
    pub bundle_id: String,
    pub display_name: Option<String>,
}

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct AppMetadataInfo {
    pub bundle_id: String,
    pub display_name: Option<String>,
    pub icon_data_url: Option<String>,
}

pub struct AppContext {
    pub device_state: Option<DeviceState>,
    pub apps_needing_relaunch: RwLock<BTreeSet<String>>,
    pub key_event_state: KeyEventState,

    // Not a mutex since it uses Atomics
    app_mode_state_machine: Arc<AppModeStateMachine>,
}

impl AppContext {
    pub fn new() -> Result<Self> {
        let device_state = if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(DeviceState {})
        } else {
            None
        };

        let app_mode_state_machine = Arc::new(AppModeStateMachine::new(device_state.clone()));

        Ok(Self {
            device_state,
            apps_needing_relaunch: RwLock::new(BTreeSet::new()),
            key_event_state: KeyEventState::new(app_mode_state_machine.clone()),
            app_mode_state_machine,
        })
    }

    pub fn get_app_mode(&self) -> AppMode {
        self.app_mode_state_machine.get_app_mode()
    }

    pub fn is_shift_pressed(&self) -> bool {
        self.app_mode_state_machine
            .is_shift_pressed
            .load(Ordering::SeqCst)
    }
}

fn normalize_bundle_id(bundle_id: &str) -> Result<String> {
    let bundle_id = bundle_id.trim();
    if bundle_id.is_empty() {
        anyhow::bail!("bundle id cannot be empty");
    }

    Ok(bundle_id.to_string())
}

fn apps_needing_relaunch_payload(bundle_ids: &BTreeSet<String>) -> Vec<AppNeedsRelaunchInfo> {
    bundle_ids
        .iter()
        .map(|bundle_id| AppNeedsRelaunchInfo {
            bundle_id: bundle_id.clone(),
            display_name: resolve_app_display_name(bundle_id),
        })
        .collect()
}

fn emit_apps_needing_relaunch_changed<R: Runtime>(
    app: &AppHandle<R>,
    bundle_ids: &BTreeSet<String>,
) -> Result<()> {
    let payload = apps_needing_relaunch_payload(bundle_ids);
    app.emit(APPS_NEEDING_RELAUNCH_CHANGED_EVENT, payload)?;
    Ok(())
}

pub fn set_app_needs_relaunch<R: Runtime>(
    app: &AppHandle<R>,
    bundle_id: &str,
    needs_relaunch: bool,
) -> Result<()> {
    let bundle_id = normalize_bundle_id(bundle_id)?;
    let context = app.app_context();

    let (changed, snapshot) = {
        let mut apps_needing_relaunch = context.apps_needing_relaunch.write();
        let changed = if needs_relaunch {
            apps_needing_relaunch.insert(bundle_id.clone())
        } else {
            apps_needing_relaunch.remove(bundle_id.as_str())
        };

        (changed, apps_needing_relaunch.clone())
    };

    if changed {
        emit_apps_needing_relaunch_changed(app, &snapshot)?;
    }

    Ok(())
}

pub fn list_apps_needing_relaunch(app: AppHandle) -> Result<Vec<AppNeedsRelaunchInfo>> {
    let context = app.app_context();
    let apps_needing_relaunch = context.apps_needing_relaunch.read();
    Ok(apps_needing_relaunch_payload(&apps_needing_relaunch))
}

pub fn get_app_metadata(bundle_id: String) -> Result<AppMetadataInfo> {
    Ok(AppMetadataInfo {
        display_name: resolve_app_display_name(&bundle_id),
        icon_data_url: resolve_app_icon_data_url(&bundle_id),
        bundle_id,
    })
}

pub fn relaunch_app(app: AppHandle, bundle_id: &str) -> Result<()> {
    let bundle_id = normalize_bundle_id(bundle_id)?;
    relaunch_bundle_id(&bundle_id)?;
    set_app_needs_relaunch(&app, &bundle_id, false)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn resolve_app_display_name(bundle_id: &str) -> Option<String> {
    let bundle_id = NSString::from_str(bundle_id);
    let running_apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);

    if let Some(app) = running_apps.iter().next() {
        if let Some(name) = app.localizedName() {
            return Some(name.to_string());
        }
    }

    let workspace = NSWorkspace::sharedWorkspace();
    let app_url = workspace.URLForApplicationWithBundleIdentifier(&bundle_id)?;
    let app_name = app_url.lastPathComponent()?;
    let app_name = app_name.to_string();
    Some(
        app_name
            .strip_suffix(".app")
            .unwrap_or(&app_name)
            .to_string(),
    )
}

#[cfg(target_os = "macos")]
fn resolve_app_path(bundle_id: &str) -> Option<String> {
    let bundle_id = NSString::from_str(bundle_id);
    let workspace = NSWorkspace::sharedWorkspace();
    let app_url = workspace.URLForApplicationWithBundleIdentifier(&bundle_id)?;
    let app_path = app_url.path()?;
    Some(app_path.to_string())
}

#[cfg(target_os = "macos")]
fn resolve_app_icon_data_url(bundle_id: &str) -> Option<String> {
    let app_path = resolve_app_path(bundle_id)?;
    let workspace = NSWorkspace::sharedWorkspace();
    let app_path = NSString::from_str(&app_path);
    let icon = workspace.iconForFile(&app_path);
    icon.setSize(NSSize::new(20.0, 20.0));

    let tiff = icon.TIFFRepresentation()?;
    let bitmap = NSBitmapImageRep::imageRepWithData(&tiff)?;
    let properties = NSDictionary::<objc2_app_kit::NSBitmapImageRepPropertyKey, AnyObject>::new();
    let png_data = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
    }?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(png_data.to_vec());
    Some(format!("data:image/png;base64,{encoded}"))
}

#[cfg(not(target_os = "macos"))]
fn resolve_app_display_name(_bundle_id: &str) -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
fn resolve_app_icon_data_url(_bundle_id: &str) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn relaunch_bundle_id(bundle_id: &str) -> Result<()> {
    let bundle_id_string = bundle_id.to_string();
    let bundle_id = NSString::from_str(bundle_id);
    let running_apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);

    for app in running_apps.iter() {
        app.terminate();
    }

    // NSRunningApplication::terminate is async, so give the app a brief window to exit
    // before asking LaunchServices to start it again.
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let still_running =
            NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);
        if still_running.is_empty() {
            break;
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    let workspace = NSWorkspace::sharedWorkspace();
    #[allow(deprecated)]
    let launched = workspace
        .launchAppWithBundleIdentifier_options_additionalEventParamDescriptor_launchIdentifier(
            &bundle_id,
            NSWorkspaceLaunchOptions::Default,
            None,
            None,
        );

    if !launched {
        anyhow::bail!("failed to relaunch app with bundle id {bundle_id_string}");
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn relaunch_bundle_id(bundle_id: &str) -> Result<()> {
    anyhow::bail!("relaunching apps is not supported on this platform: {bundle_id}");
}

// Load all the modules specified in the config.js.module of the `macos.toml` files.
pub async fn load_chord_files_runtime_modules(
    handle: AppHandle,
    loaded_app_chords: &ChordRegistry,
) {
    for (bundle_id, runtime) in loaded_app_chords.runtimes.iter() {
        let handle = handle.clone();

        let Some(js) = runtime.config.as_ref().and_then(|c| c.js.as_ref()) else {
            continue;
        };

        let Some(content) = js.module.clone() else {
            continue;
        };

        let path = runtime.path.clone();
        let raw_chords = runtime.raw_chords.lock().unwrap().clone();
        let bundle_id = bundle_id.clone();

        tauri::async_runtime::spawn(async move {
            let path_ = path.clone();
            let result = with_js(handle, move |ctx| {
                Box::pin(async move {
                    let load_module = || -> rquickjs::Result<rquickjs::Promise> {
                        let module = Module::declare(ctx.clone(), path.clone(), content)?;
                        let chords =
                            rquickjs_serde::to_value(ctx.clone(), raw_chords).or_throw(&ctx)?;
                        let chords_obj = chords.into_object().or_throw(&ctx);
                        let meta = module.meta()?;
                        meta.set("chords", chords_obj)?;
                        meta.set("bundleId", bundle_id)?;
                        let (_evaluated, promise) = module.eval()?;
                        Ok(promise)
                    };

                    match load_module() {
                        Ok(promise) => {
                            if let Err(e) = promise.into_future::<()>().await {
                                log::error!(
                                    "failed to await module {}: {}",
                                    path,
                                    format_js_error(ctx.clone(), e)
                                )
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to load module {}: {}",
                                path,
                                format_js_error(ctx.clone(), e)
                            );
                        }
                    }

                    Ok(())
                })
            })
            .await;

            if let Err(err) = result {
                log::error!("load_module failed for {}: {}", path_, err);
            }
        });
    }
}

fn format_action(chord: &crate::chords::Chord) -> String {
    if let Some(shortcut) = &chord.shortcut {
        return format!("Shortcut: {}", format_shortcut(shortcut));
    }

    if let Some(shell) = &chord.shell {
        return format!("Shell: {shell}");
    }

    if let Some(js) = &chord.js {
        let export_name = js.export_name.as_deref().unwrap_or("default");
        let args = match &js.args {
            crate::chords::ChordJsArgs::Values(values) => format!("{values:?}"),
            crate::chords::ChordJsArgs::Eval(source) => format!("<eval: {source}>"),
        };
        return format!("JS: {}({})", export_name, args);
    }

    "No action".to_string()
}

fn format_shortcut(shortcut: &crate::chords::Shortcut) -> String {
    shortcut
        .chords
        .iter()
        .map(|chord| {
            chord
                .keys
                .iter()
                .map(|key| format_key(*key))
                .collect::<Vec<_>>()
                .join(" + ")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_sequence(keys: &[crate::input::Key]) -> String {
    keys.iter()
        .map(|key| {
            key.to_char(false)
                .map(|ch| ch.to_ascii_uppercase().to_string())
                .unwrap_or_else(|| format_key(*key))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_key(key: crate::input::Key) -> String {
    if let Some(ch) = key.to_char(false) {
        return ch.to_ascii_uppercase().to_string();
    }

    match key.0 {
        ShiftLeft | ShiftRight => "Shift".to_string(),
        ControlLeft | ControlRight => "Ctrl".to_string(),
        MetaLeft | MetaRight => "Cmd".to_string(),
        AltLeft | AltRight => "Alt".to_string(),
        CapsLock => "Caps Lock".to_string(),
        Space => "Space".to_string(),
        Enter => "Enter".to_string(),
        Tab => "Tab".to_string(),
        Escape => "Esc".to_string(),
        ArrowUp => "Up".to_string(),
        ArrowDown => "Down".to_string(),
        ArrowLeft => "Left".to_string(),
        ArrowRight => "Right".to_string(),
        Backspace => "Backspace".to_string(),
        Delete => "Delete".to_string(),
        Home => "Home".to_string(),
        End => "End".to_string(),
        PageUp => "Page Up".to_string(),
        PageDown => "Page Down".to_string(),
        Fn => "Fn".to_string(),
        other => format!("{other:?}"),
    }
}
