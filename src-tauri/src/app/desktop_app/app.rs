use crate::app::AppSingleton;
use crate::app::desktop_app::DesktopAppManager;
use crate::state::{DesktopAppManagerObservable, Observable};
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct DesktopAppManagerProvider {
    pub desktop_app_manager_observable: DesktopAppManagerObservable,
    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for DesktopAppManager {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
