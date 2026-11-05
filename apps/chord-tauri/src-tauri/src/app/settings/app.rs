use crate::app::settings::settings::AppSettings;
use crate::app::{AppHandleExt, AppSingleton};
use crate::state::{AppPermissionsState, AppSettingsObservable};
use anyhow::Result;
use nject::provider;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[provider]
pub struct AppSettingsProvider {
    #[provide(AppSettingsObservable, |v| v.provide())]
    pub app_settings_observable: AppSettingsObservable,

    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for AppSettings {
    fn init(&self) -> Result<()> {
        if let Err(error) = self.refresh_cli_installation() {
            log::warn!("Could not refresh command line installation: {error:#}");
        }

        let state = self.handle.app_state();
        let repos_store = &state.chord_package_manager().registry.git.git_repos_store;
        self.load_onboarding(repos_store.has_persisted_state()?)?;

        // Whoever just switched Chord on is looking at System Settings. During setup, bring
        // them back to the next step instead of leaving them to find their way.
        let permissions = self
            .handle
            .state::<observable_property::ObservableProperty<AppPermissionsState>>();
        let handle = self.handle.clone();
        permissions.subscribe(Arc::new(move |previous, current| {
            let granted = previous.is_accessibility_enabled != Some(true)
                && current.is_accessibility_enabled == Some(true);
            let settings = handle.app_state().settings();
            if granted && !settings.is_onboarding_complete() {
                if let Err(error) = settings.ui.open() {
                    log::error!("Failed to bring setup forward after the grant: {error:#}");
                }
            }
        }))?;
        Ok(())
    }
}
