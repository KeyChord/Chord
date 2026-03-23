use anyhow::Result;
use std::sync::Arc;
use crate::feature::SafeAppHandle;
use crate::input::{register_caps_lock_input_handler, register_key_event_input_grabber};
use crate::observables::{AppPermissionsObservable, AppPermissionsState};

pub struct AppPermissions {
    _observable: AppPermissionsObservable,
    _input_monitoring: AppPermissionsInputMonitoring,
    _accessibility: AppPermissionsAccessibility
}

impl AppPermissions {
    pub async fn from_check(safe_handle: SafeAppHandle) -> Result<Self> {
        let state = AppPermissionsState {
            is_autostart_enabled: safe_handle.is_autolaunch_enabled()?,
            is_input_monitoring_enabled: tauri_plugin_macos_permissions::check_input_monitoring_permission().await,
            is_accessibility_enabled: tauri_plugin_macos_permissions::check_accessibility_permission().await,
        };
        let observable = AppPermissionsObservable::new(safe_handle.clone(), state)?;
        let input_monitoring = AppPermissionsInputMonitoring::new_from_observable(safe_handle.clone(), &observable)?;
        let accessibility = AppPermissionsAccessibility::new_from_observable(safe_handle.clone(), &observable)?;
        Ok(Self {
            _observable: observable,
            _input_monitoring: input_monitoring,
            _accessibility: accessibility
        })
    }
}

pub struct AppPermissionsInputMonitoring {}

impl AppPermissionsInputMonitoring {
    pub fn new_from_observable(safe_handle: SafeAppHandle, observable: &AppPermissionsObservable) -> Result<Self> {
        let on_input_monitoring_enabled = move || {
            safe_handle.on_safe(|handle| {
                if let Err(e) = register_caps_lock_input_handler(handle.clone()) {
                    log::error!("Failed to handle caps lock input: {e}");
                }
            });
        };

        let state = observable.get_state()?;
        if state.is_input_monitoring_enabled {
            on_input_monitoring_enabled();
        } else {
            observable.subscribe(Arc::new(move |_, state| {
                if state.is_input_monitoring_enabled {
                    on_input_monitoring_enabled();
                }
            }))?;
        }


        Ok(Self {})
    }
}

pub struct AppPermissionsAccessibility {}

impl AppPermissionsAccessibility {
    pub fn new_from_observable(safe_handle: SafeAppHandle, observable: &AppPermissionsObservable) -> Result<Self> {
        let on_accessibility_enabled = move || {
            safe_handle.on_safe(|handle| {
                register_key_event_input_grabber(handle.clone());
            });
        };

        let state = observable.get_state()?;
        if state.is_accessibility_enabled {
            on_accessibility_enabled();
        } else {
            observable.subscribe(Arc::new(move |_, state| {
                if state.is_accessibility_enabled {
                    on_accessibility_enabled();
                }
            }))?;
        }

        Ok(Self {})
    }
}
