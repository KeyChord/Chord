use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use crate::app::AppSingleton;
use crate::app::permissions::{AppPermissions, AppPermissionsAccessibility, AppPermissionsInputMonitoring};
use crate::state::{AppPermissionsObservable, AppPermissionsState, Observable};

impl AppSingleton<AppPermissionsObservable> for AppPermissions {
    fn new(handle: AppHandle) -> Self {
        Self {
            input_monitoring: AppPermissionsInputMonitoring::new(handle.clone()),
            accessibility: AppPermissionsAccessibility::new(handle.clone()),
            observable: AppPermissionsObservable::uninitialized(),
            handle,
        }
    }

    fn init(&self, observable: AppPermissionsObservable) -> anyhow::Result<()> {
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

}

