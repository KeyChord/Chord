use crate::input::{register_caps_lock_input_handler, register_key_event_input_grabber};
use crate::observables::{AppPermissionsObservable, AppPermissionsState, Observable};
use anyhow::{Context, Result};
use std::sync::mpsc;
use std::sync::Arc;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;
use crate::app::state::StateSingleton;

pub struct AppPermissions {
    input_monitoring: AppPermissionsInputMonitoring,
    accessibility: AppPermissionsAccessibility,

    observable: AppPermissionsObservable,
    handle: AppHandle,
}

impl StateSingleton for AppPermissions {
    fn new(handle: AppHandle) -> Self {
        Self {
            input_monitoring: AppPermissionsInputMonitoring::new(handle.clone()),
            accessibility: AppPermissionsAccessibility::new(handle.clone()),
            observable: AppPermissionsObservable::placeholder(),
            handle,
        }
    }
}

impl AppPermissions {
    pub fn init(&self, observable: AppPermissionsObservable) -> Result<()> {
        self.observable.init(observable);

        self.input_monitoring.init(&self.observable)?;
        self.accessibility.init(&self.observable)?;
        let is_autolaunch_enabled = self.handle.autolaunch().is_enabled()?;
        self.observable.set_state(AppPermissionsState {
            is_input_monitoring_enabled: None,
            is_accessibility_enabled: None,
            is_autostart_enabled: Some(is_autolaunch_enabled),
        })?;
        Ok(())
    }

    pub async fn load(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        let new_state = AppPermissionsState {
            is_input_monitoring_enabled: Some(
                tauri_plugin_macos_permissions::check_input_monitoring_permission().await,
            ),
            is_accessibility_enabled: Some(
                tauri_plugin_macos_permissions::check_accessibility_permission().await,
            ),
            is_autostart_enabled: state.is_autostart_enabled,
        };
        log::debug!("loaded app permissions: {:?}", new_state);
        self.observable.set_state(new_state)?;
        Ok(())
    }

    pub fn toggle_autostart(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        let Some(is_autostart_enabled) = state.is_autostart_enabled else {
            anyhow::bail!("app not ready")
        };
        if is_autostart_enabled {
            self.handle.autolaunch().disable()?;
            self.observable.set_state(AppPermissionsState {
                is_autostart_enabled: Some(false),
                ..*state
            })?;
        } else {
            self.handle.autolaunch().enable()?;
            self.observable.set_state(AppPermissionsState {
                is_autostart_enabled: Some(true),
                ..*state
            })?;
        }
        Ok(())
    }
}

pub struct AppPermissionsInputMonitoring {
    handle: AppHandle
}

impl AppPermissionsInputMonitoring {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    pub fn init(&self, observable: &AppPermissionsObservable) -> Result<()> {
        let handle = self.handle.clone();
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_input_monitoring_enabled.is_some_and(|s| s) {
                if let Err(e) = register_caps_lock_input_handler(handle.clone()) {
                    log::error!("Failed to handle caps lock input: {e}");
                }
            }
        }))?;
        Ok(())
    }
}

pub struct AppPermissionsAccessibility {
    handle: AppHandle
}

impl AppPermissionsAccessibility {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    pub fn init(&self, observable: &AppPermissionsObservable) -> Result<()> {
        let handle = self.handle.clone();
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_accessibility_enabled.is_some_and(|s| s) {
                register_key_event_input_grabber(handle.clone());
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
