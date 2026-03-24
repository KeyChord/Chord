use anyhow::Result;
use std::sync::Arc;
use crate::feature::SafeAppHandle;
use crate::input::{register_caps_lock_input_handler, register_key_event_input_grabber};
use crate::observables::{AppPermissionsObservable, AppPermissionsState, AppSettingsObservable, Observable};

pub struct AppPermissions {
    _input_monitoring: AppPermissionsInputMonitoring,
    _accessibility: AppPermissionsAccessibility,

    handle: SafeAppHandle
}

impl AppPermissions {
    pub fn new_unloaded(handle: SafeAppHandle) -> Result<Self> {
        let input_monitoring = AppPermissionsInputMonitoring::new(handle.clone())?;
        let accessibility = AppPermissionsAccessibility::new(handle.clone())?;
        let is_autolaunch_enabled = handle.is_autolaunch_enabled()?;
        let observable = handle.observable::<AppPermissionsObservable>();
        observable.set_state(AppPermissionsState {
            is_input_monitoring_enabled: None,
            is_accessibility_enabled: None,
            is_autostart_enabled: Some(is_autolaunch_enabled)
        })?;
        Ok(Self {
            handle,
            _input_monitoring: input_monitoring,
            _accessibility: accessibility
        })
    }

    pub async fn load(&self) -> Result<()> {
        let observable = self.handle.observable::<AppPermissionsObservable>();
        let state = observable.get_state()?;
        observable.set_state(AppPermissionsState {
            is_input_monitoring_enabled: Some(tauri_plugin_macos_permissions::check_input_monitoring_permission().await),
            is_accessibility_enabled: Some(tauri_plugin_macos_permissions::check_accessibility_permission().await),
            is_autostart_enabled: state.is_autostart_enabled
        })?;
        Ok(())
    }
}

pub struct AppPermissionsInputMonitoring {}

impl AppPermissionsInputMonitoring {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let observable = handle.observable::<AppPermissionsObservable>();
        let handle = handle.clone();
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_input_monitoring_enabled.is_some_and(|s| s) {
                handle.on_safe(|handle| {
                    if let Err(e) = register_caps_lock_input_handler(handle.clone()) {
                        log::error!("Failed to handle caps lock input: {e}");
                    }
                });
            }
        }))?;

        Ok(Self {})
    }
}

pub struct AppPermissionsAccessibility {}

impl AppPermissionsAccessibility {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let observable = handle.observable::<AppPermissionsObservable>();
        let handle = handle.clone();
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_accessibility_enabled.is_some_and(|s| s) {
                handle.on_safe(|handle| {
                    register_key_event_input_grabber(handle.clone());
                });
            }
        }))?;

        Ok(Self {})
    }
}
