use crate::app::AppSingleton;
use crate::app::settings::settings::AppSettings;
use crate::state::AppSettingsObservable;
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct AppSettingsProvider {
    #[provide(AppSettingsObservable, |v| v.provide())]
    pub app_settings_observable: AppSettingsObservable,

    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for AppSettings {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
