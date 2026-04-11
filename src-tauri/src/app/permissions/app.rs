use crate::app::AppSingleton;
use crate::app::permissions::{
    AppPermissions, AppPermissionsAccessibility, AppPermissionsInputMonitoring,
};
use crate::state::{AppPermissionsObservable, AppPermissionsState, Observable};
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[provider]
pub struct AppPermissionsProvider {
    #[provide(AppHandle, |h| h.clone())]
    pub handle: AppHandle,

    pub app_permissions_observable: AppPermissionsObservable,
}

impl AppSingleton for AppPermissions {
    fn init(&self) -> Result<()> {
        self.init()
    }
}
