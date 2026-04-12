use crate::app::AppSingleton;
use crate::state::{AppModeObservable, Observable};
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;
use crate::app::controller::AppController;

#[provider]
pub struct AppControllerProvider {
    #[provide(AppModeObservable, |v| v.provide())]
    pub app_mode_observable: AppModeObservable,
    
    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for AppController {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
