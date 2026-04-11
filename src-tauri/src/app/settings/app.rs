use crate::app::AppSingleton;
use crate::app::settings::settings::AppSettings;
use crate::app::settings::settings_ui::SettingsUi;
use crate::state::{AppSettingsObservable, Observable};
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct AppSettingsProvider {
    #[provide(AppHandle, |h| h.clone())]
    pub handle: AppHandle,

    pub app_settings_observable: AppSettingsObservable,
}

impl AppSingleton for AppSettings {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
