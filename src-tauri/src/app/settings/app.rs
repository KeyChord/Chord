use tauri::AppHandle;
use crate::app::AppSingleton;
use crate::app::settings::settings::AppSettings;
use crate::app::settings::settings_ui::SettingsUi;
use crate::state::{AppSettingsObservable, Observable};

impl AppSingleton<AppSettingsObservable> for AppSettings {
    fn new(handle: AppHandle) -> Self {
        Self {
            ui: SettingsUi::new(handle.clone()),
            observable: AppSettingsObservable::uninitialized(),
            handle,
        }
    }

    fn init(&self, observable: AppSettingsObservable) -> anyhow::Result<()> {
        self.observable.init(observable);
        Ok(())
    }
}
