use crate::app::AppSingleton;
use crate::app::mode::AppModeManager;
use crate::state::{AppModeObservable, Observable};
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct AppModeManagerProvider {
    pub app_mode_observable: AppModeObservable,
    #[provide(AppHandle, |h| h.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for AppModeManager {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
