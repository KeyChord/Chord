use crate::chords::{Chord, ChordPackage, LoadedAppChords, GLOBAL_CHORD_RUNTIME_ID};
use crate::js::{format_js_error, reset_js, with_js};
use crate::feature::app_handle_ext::AppHandleExt;
use crate::{
    input::KeyEventState,
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::Result;
use base64::Engine;
use device_query::DeviceState;
use keycode::KeyMappingCode::*;
use parking_lot::RwLock;
use rquickjs::Module;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSRunningApplication, NSWorkspace,
    NSWorkspaceLaunchOptions,
};
use objc2_foundation::{NSDictionary, NSSize, NSString};
use std::time::{Duration, Instant};

const APPS_NEEDING_RELAUNCH_CHANGED_EVENT: &str = "apps-needing-relaunch-changed";

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct ActiveChordInfo {
    pub scope: String,
    pub scope_kind: String,
    pub sequence: String,
    pub name: String,
    pub action: String,
    pub description: Option<String>,
    pub is_description: bool,
}

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
    pub loaded_app_chords: RwLock<LoadedAppChords>,
    pub apps_needing_relaunch: RwLock<BTreeSet<String>>,
    pub key_event_state: KeyEventState,

    // Not a mutex since it uses Atomics
    app_mode_state_machine: Arc<AppModeStateMachine>,
}

impl AppContext {
    pub fn new() -> Result<Self> {
        let bundled_app_chords = LoadedAppChords::from_folders(vec![ChordPackage::load_bundled()?])?;

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
            loaded_app_chords: RwLock::new(bundled_app_chords),
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
    Ok( AppMetadataInfo {
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

fn module_disk_path(root_dir: Option<&Path>, module_path: &str) -> String {
    root_dir
        .map(|root_dir| root_dir.join(module_path))
        .unwrap_or_else(|| PathBuf::from(module_path))
        .display()
        .to_string()
}

// Also evaluates JavaScript
pub async fn reload_loaded_app_chords(app: AppHandle) -> Result<()> {
    let context = app.app_context();
    let chorder = app.app_chorder();
    let chord_package_registry = app.app_chord_package_registry();
    chorder.ensure_inactive()?;

    // Load all JS files as modules
    let chord_folders = chord_package_registry.load_all_chord_packages()?;
    reset_js(app.clone()).await?;

    // Load all JS files as modules, but keep `chord_folders` so we can use it later.
    for chord_folder in &chord_folders {
        let js_files = chord_folder.js_files.clone();
        let root_dir = chord_folder.root_dir.clone();

        with_js(app.clone(), move |ctx| {
            Box::pin(async move {
                for (filepath, js) in js_files {
                    let module_disk_path = module_disk_path(root_dir.as_deref(), &filepath);
                    let module = match Module::declare(ctx.clone(), filepath.clone(), js) {
                        Ok(m) => {
                            log::debug!("Declared module {}", filepath);
                            m
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to declare JS module {}: {}",
                                filepath,
                                format_js_error(ctx.clone(), e)
                            );
                            continue;
                        }
                    };

                    let meta = match module.meta() {
                        Ok(meta) => meta,
                        Err(e) => {
                            log::error!(
                                "Failed to get import.meta for JS module {}: {}",
                                filepath,
                                format_js_error(ctx.clone(), e)
                            );
                            continue;
                        }
                    };

                    if let Err(e) = meta.set("url", module_disk_path) {
                        log::error!(
                            "Failed to set import.meta.url for JS module {}: {}",
                            filepath,
                            format_js_error(ctx.clone(), e)
                        );
                        continue;
                    }

                    let (_evaluated, promise) = match module.eval() {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!(
                                "Failed to start evaluating JS module {}: {}",
                                filepath,
                                format_js_error(ctx.clone(), e)
                            );
                            continue;
                        }
                    };

                    if let Err(e) = promise.into_future::<()>().await {
                        log::error!(
                            "Failed to evaluate JS module {}: {}",
                            filepath,
                            format_js_error(ctx.clone(), e)
                        );
                    }
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    }

    let loaded_chords = LoadedAppChords::from_folders(chord_folders)?;
    // We should only load `macos.toml` modules AFTER the js files have been loaded
    load_chord_files_runtime_modules(app.clone(), &loaded_chords).await;

    log::debug!("Loaded chord files: {:?}", loaded_chords.runtimes.keys());
    *context.loaded_app_chords.write() = loaded_chords;

    Ok(())
}

// Load all the modules specified in the config.js.module of the `macos.toml` files.
pub async fn load_chord_files_runtime_modules(
    handle: AppHandle,
    loaded_app_chords: &LoadedAppChords,
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
                    let module = match Module::declare(ctx.clone(), path.clone(), content) {
                        Ok(m) => m,
                        Err(e) => {
                            log::error!(
                                "Failed to declare module {}: {}",
                                path,
                                format_js_error(ctx.clone(), e)
                            );
                            return Ok(());
                        }
                    };

                    let chords = match rquickjs_serde::to_value(ctx.clone(), raw_chords) {
                        Ok(value) => value,
                        Err(error) => {
                            log::error!("Failed to serialize chords: {error}");
                            return Ok(());
                        }
                    };

                    let chords_obj = match chords.into_object() {
                        Some(value) => value,
                        None => {
                            log::error!("Failed to convert chords to object");
                            return Ok(());
                        }
                    };

                    let meta = match module.meta() {
                        Ok(meta) => meta,
                        Err(error) => {
                            log::error!("Failed to get import.meta for module {}: {error}", path);
                            return Ok(());
                        }
                    };

                    if let Err(e) = meta.set("chords", chords_obj) {
                        log::error!(
                            "Failed to set `import.meta.chords` for module {}: {}",
                            path,
                            format_js_error(ctx.clone(), e)
                        );
                        return Ok(());
                    }

                    if let Err(e) = meta.set("bundleId", bundle_id) {
                        log::error!(
                            "Failed to set `import.meta.bundleId` for module {}: {}",
                            path,
                            format_js_error(ctx.clone(), e)
                        );
                        return Ok(());
                    }

                    let (_evaluated, promise) = match module.eval() {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!(
                                "Failed to start evaluating module {}: {}",
                                path,
                                format_js_error(ctx.clone(), e)
                            );
                            return Ok(());
                        }
                    };

                    if let Err(e) = promise.into_future::<()>().await {
                        log::error!(
                            "Failed to evaluate module {}: {}",
                            path,
                            format_js_error(ctx.clone(), e)
                        );
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

pub fn list_active_chords(app: AppHandle) -> Result<Vec<ActiveChordInfo>> {
    let context = app.app_context();
    let loaded_app_chords = context.loaded_app_chords.read();
    Ok(list_loaded_chords(&loaded_app_chords))
}

pub fn list_matching_chords(app: AppHandle) -> Result<Vec<ActiveChordInfo>> {
    let context = app.app_context();
    let chorder = app.app_chorder();
    let frontmost = app.app_frontmost();
    let state = chorder.observable.get_state()?;
    let frontmost_application_id = frontmost.frontmost_application_id.load().as_ref().clone();
    let loaded_app_chords = context.loaded_app_chords.read();

    Ok(list_matching_loaded_chords(
        &loaded_app_chords,
        &state.key_buffer,
        frontmost_application_id.as_deref(),
    ))
}

pub fn list_loaded_chords(loaded_app_chords: &LoadedAppChords) -> Vec<ActiveChordInfo> {
    let mut chords = Vec::new();
    let mut seen = HashSet::new();

    for (application_id, runtime) in &loaded_app_chords.runtimes {
        for chord in runtime.chords.values() {
            let item = build_active_chord_info(
                application_scope(application_id),
                application_scope_kind(application_id),
                &chord.keys,
                chord,
            );
            let fingerprint = format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                item.scope_kind, item.scope, item.sequence, item.name, item.action
            );
            if seen.insert(fingerprint) {
                chords.push(item);
            }
        }
    }

    chords.sort_by(|left, right| {
        left.scope_kind
            .cmp(&right.scope_kind)
            .then(left.scope.cmp(&right.scope))
            .then(left.sequence.cmp(&right.sequence))
            .then(left.name.cmp(&right.name))
    });

    chords
}

pub fn list_matching_loaded_chords(
    loaded_app_chords: &LoadedAppChords,
    key_buffer: &[crate::input::Key],
    application_id: Option<&str>,
) -> Vec<ActiveChordInfo> {
    let mut items = loaded_app_chords
        .list_matching_descriptions(key_buffer, application_id)
        .into_iter()
        .map(|item| {
            build_description_info(
                &item.scope,
                item.scope_kind,
                &item.sequence,
                &item.description,
            )
        })
        .collect::<Vec<_>>();

    items.extend(
        loaded_app_chords
            .list_matching_chords(key_buffer, application_id)
            .into_iter()
            .map(|item| {
                build_active_chord_info(&item.scope, item.scope_kind, &item.sequence, &item.chord)
            }),
    );

    items
}

fn application_scope(application_id: &str) -> &str {
    if application_id == GLOBAL_CHORD_RUNTIME_ID {
        "Global"
    } else {
        application_id
    }
}

fn application_scope_kind(application_id: &str) -> &'static str {
    if application_id == GLOBAL_CHORD_RUNTIME_ID {
        "global"
    } else {
        "app"
    }
}

fn build_active_chord_info(
    scope: &str,
    scope_kind: &str,
    sequence: &[crate::input::Key],
    chord: &Chord,
) -> ActiveChordInfo {
    ActiveChordInfo {
        scope: scope.to_string(),
        scope_kind: scope_kind.to_string(),
        sequence: format_sequence(sequence),
        name: chord.name.clone(),
        action: format_action(chord),
        description: None,
        is_description: false,
    }
}

fn build_description_info(
    scope: &str,
    scope_kind: &str,
    sequence: &[crate::input::Key],
    description: &str,
) -> ActiveChordInfo {
    ActiveChordInfo {
        scope: scope.to_string(),
        scope_kind: scope_kind.to_string(),
        sequence: format_sequence(sequence),
        name: description.to_string(),
        action: "Description".to_string(),
        description: Some(description.to_string()),
        is_description: true,
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
