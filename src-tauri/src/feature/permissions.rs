use crate::feature::SafeAppHandle;
use crate::input::{register_caps_lock_input_handler, register_key_event_input_grabber};
use crate::observables::{
    AppPermissionsObservable, AppPermissionsState, AppSettingsObservable, Observable,
};
use anyhow::Result;
use std::sync::Arc;

pub struct AppPermissions {
    _input_monitoring: AppPermissionsInputMonitoring,
    _accessibility: AppPermissionsAccessibility,

    observable: Arc<AppPermissionsObservable>,
    handle: SafeAppHandle,
}

impl AppPermissions {
    pub fn new_unloaded(
        handle: SafeAppHandle,
        observable: Arc<AppPermissionsObservable>,
    ) -> Result<Self> {
        let input_monitoring =
            AppPermissionsInputMonitoring::new(handle.clone(), observable.clone())?;
        let accessibility = AppPermissionsAccessibility::new(handle.clone(), observable.clone())?;
        let is_autolaunch_enabled = handle.autolaunch().is_enabled()?;
        observable.set_state(AppPermissionsState {
            is_input_monitoring_enabled: None,
            is_accessibility_enabled: None,
            is_autostart_enabled: Some(is_autolaunch_enabled),
        })?;
        Ok(Self {
            handle,
            observable,
            _input_monitoring: input_monitoring,
            _accessibility: accessibility,
        })
    }

    pub async fn load(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        self.observable.set_state(AppPermissionsState {
            is_input_monitoring_enabled: Some(
                tauri_plugin_macos_permissions::check_input_monitoring_permission().await,
            ),
            is_accessibility_enabled: Some(
                tauri_plugin_macos_permissions::check_accessibility_permission().await,
            ),
            is_autostart_enabled: state.is_autostart_enabled,
        })?;
        Ok(())
    }

    pub fn enable_autostart(&self) -> Result<()> {
        Ok(self.handle.autolaunch().enable()?)
    }
}

pub struct AppPermissionsInputMonitoring {}

impl AppPermissionsInputMonitoring {
    pub fn new(handle: SafeAppHandle, observable: Arc<AppPermissionsObservable>) -> Result<Self> {
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
    pub fn new(handle: SafeAppHandle, observable: Arc<AppPermissionsObservable>) -> Result<Self> {
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
