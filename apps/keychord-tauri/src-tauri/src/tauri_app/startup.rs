use anyhow::{Context, Result};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

pub(crate) const APP_STATE_STORE_PATH: &str = "app-state.json";
const ONBOARDING_COMPLETED_KEY: &str = "onboardingCompleted";

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct StartupStatusInfo {
    pub launched_via_autostart: bool,
    pub onboarding_completed: bool,
    pub should_show_onboarding: bool,
}

fn read_onboarding_completed<R: Runtime>(app: &AppHandle<R>) -> Result<bool> {
    let store = app
        .store(APP_STATE_STORE_PATH)
        .context("failed to open app state store")?;

    let value = store.get(ONBOARDING_COMPLETED_KEY);
    match value {
        Some(value) => serde_json::from_value::<bool>(value)
            .context("failed to parse onboarding completed flag"),
        None => Ok(false),
    }
}

pub fn is_autostart_launch() -> bool {
    std::env::args_os().any(|arg| arg == "--autostart")
}

/// macOS permission checks are async on Tauri's runtime; `tauri::async_runtime::block_on` must not
/// run on a Tokio worker (e.g. from an IPC handler), so we isolate checks on a short-lived runtime.
fn required_permissions_granted_sync() -> bool {
    match std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build permission-check runtime");
        rt.block_on(async {
            tauri_plugin_macos_permissions::check_input_monitoring_permission().await
                && tauri_plugin_macos_permissions::check_accessibility_permission().await
        })
    })
    .join()
    {
        Ok(granted) => granted,
        Err(_) => {
            log::warn!("permission check thread panicked; treating permissions as not granted");
            false
        }
    }
}

pub fn get_startup_status<R: Runtime>(app: &AppHandle<R>) -> Result<StartupStatusInfo> {
    let launched_via_autostart = is_autostart_launch();
    let onboarding_completed = read_onboarding_completed(app)?;

    let required_permissions_granted = required_permissions_granted_sync();

    Ok(StartupStatusInfo {
        launched_via_autostart,
        onboarding_completed,
        should_show_onboarding: !launched_via_autostart
            && (!onboarding_completed || !required_permissions_granted),
    })
}

pub fn complete_onboarding<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let store = app
        .store(APP_STATE_STORE_PATH)
        .context("failed to open app state store")?;
    store.set(ONBOARDING_COMPLETED_KEY, true);
    store.save().context("failed to save app state store")?;
    Ok(())
}
