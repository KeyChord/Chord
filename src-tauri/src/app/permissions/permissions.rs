use crate::app::state::AppSingleton;
use crate::state::{AppPermissionsObservable, AppPermissionsState, Observable};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::sync::mpsc;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;
use crate::app::AppHandleExt;

pub struct AppPermissions {
    pub(super) input_monitoring: AppPermissionsInputMonitoring,
    pub(super) accessibility: AppPermissionsAccessibility,

    pub(super) observable: AppPermissionsObservable,
    pub(super) handle: AppHandle,
}

impl AppPermissions {
    pub async fn load(&self) -> Result<()> {
        let is_input_monitoring_enabled = tauri_plugin_macos_permissions::check_input_monitoring_permission().await;
        let is_accessibility_enabled = tauri_plugin_macos_permissions::check_accessibility_permission().await;
        self.observable.set_state(|prev| AppPermissionsState {
            is_input_monitoring_enabled: Some(is_input_monitoring_enabled),
            is_accessibility_enabled: Some(is_accessibility_enabled),
            ..prev
        })?;
        Ok(())
    }

    pub fn toggle_autostart(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        let Some(is_autostart_enabled) = state.is_autostart_enabled else {
            anyhow::bail!("app not ready")
        };
        if is_autostart_enabled {
            self.handle.autolaunch().disable()?;
            self.observable.set_state(|prev| AppPermissionsState {
                is_autostart_enabled: Some(false),
                ..prev
            })?;
        } else {
            self.handle.autolaunch().enable()?;
            self.observable.set_state(|prev| AppPermissionsState {
                is_autostart_enabled: Some(true),
                ..prev
            })?;
        }
        Ok(())
    }
}

pub struct AppPermissionsInputMonitoring {
    handle: AppHandle,
}

impl AppPermissionsInputMonitoring {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    pub fn init(&self, observable: &AppPermissionsObservable) -> Result<()> {
        let handle = self.handle.clone();
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_input_monitoring_enabled.is_some_and(|s| s) {
                let app = handle.state();
                let keyboard = handle.state().keyboard();
                if let Err(e) = keyboard.register_caps_lock_input_handler() {
                    log::error!("Failed to handle caps lock input: {e}");
                }
            }
        }))?;
        Ok(())
    }
}

pub struct AppPermissionsAccessibility {
    handle: AppHandle,
}

impl AppPermissionsAccessibility {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    pub fn init(&self, observable: &AppPermissionsObservable) -> Result<()> {
        let handle = self.handle.clone();
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_accessibility_enabled.is_some_and(|s| s) {
                let keyboard = handle.state().keyboard();
                keyboard.register_input_handler();
            }
        }))?;
        Ok(())
    }
}

pub fn open_system_settings(url: &str, permission_name: &str) {
    if let Err(error) = std::process::Command::new("open").arg(url).spawn() {
        log::error!("Failed to open {permission_name} settings: {error}");
    }
}

fn run_on_main_thread_sync<R: Runtime, F>(handle: &AppHandle<R>, f: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    let (tx, rx) = mpsc::sync_channel(0);
    handle
        .run_on_main_thread(move || {
            f();
            let _ = tx.send(());
        })
        .context("schedule main-thread dispatch")?;
    rx.recv()
        .context("main thread finished without signaling (channel closed)")?;
    Ok(())
}

/// `AXIsProcessTrustedWithOptions` must run on the main thread; otherwise the prompt can appear but
/// TCC may not associate the app correctly with the bundle UI in System Settings.
#[cfg(target_os = "macos")]
pub fn request_accessibility_prompt_sync<R: Runtime>(handle: &AppHandle<R>) -> Result<()> {
    log::info!(
        "Requesting accessibility trust for executable {:?}",
        std::env::current_exe().ok()
    );
    run_on_main_thread_sync(handle, || {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    })
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility_prompt_sync<R: Runtime>(_handle: &AppHandle<R>) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDRequestAccess(request: u32);
}

/// Registers this process for Input Monitoring (`kIOHIDRequestTypeListenEvent`) on the main thread.
pub fn register_input_monitoring_sync<R: Runtime>(handle: &AppHandle<R>) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        run_on_main_thread_sync(handle, || unsafe {
            IOHIDRequestAccess(1);
        })?;
    }
    Ok(())
}
