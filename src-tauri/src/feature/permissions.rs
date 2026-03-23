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
    pub async fn new(safe_handle: SafeAppHandle) -> Result<Self> {
        let observable = AppPermissionsObservable::new(safe_handle.clone())?;
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
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_input_monitoring_enabled.is_some_and(|s| s) {
                safe_handle.on_safe(|handle| {
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
    pub fn new_from_observable(safe_handle: SafeAppHandle, observable: &AppPermissionsObservable) -> Result<Self> {
        let state = observable.get_state()?;
        observable.subscribe(Arc::new(move |_, state| {
            if state.is_accessibility_enabled.is_some_and(|s| s) {
                safe_handle.on_safe(|handle| {
                    register_key_event_input_grabber(handle.clone());
                });
            }
        }))?;

        Ok(Self {})
    }
}
