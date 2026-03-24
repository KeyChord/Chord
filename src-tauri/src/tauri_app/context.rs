use crate::feature::app_handle_ext::AppHandleExt;
use crate::{
    input::{KeyEvent, KeyEventState},
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::Result;
use base64::Engine;
use device_query::DeviceState;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSRunningApplication, NSWorkspace,
    NSWorkspaceLaunchOptions,
};
use objc2_foundation::{NSDictionary, NSSize, NSString};
use parking_lot::RwLock;
use std::collections::BTreeSet;
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

    pub fn take_caps_lock_passthrough_on_release(&self, event: &KeyEvent) -> bool {
        self.app_mode_state_machine
            .take_caps_lock_passthrough_on_release(event)
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
