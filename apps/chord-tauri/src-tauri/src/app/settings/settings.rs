use crate::app::settings::settings_ui::SettingsUi;
use crate::startup::APP_STATE_STORE_PATH;
use crate::state::{AppSettingsObservable, AppSettingsState, Observable};
use crate::tray::TRAY_ID;
use anyhow::Context;
use nject::injectable;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

#[injectable]
pub struct AppSettings {
    pub ui: SettingsUi,
    observable: AppSettingsObservable,
    pub(super) handle: AppHandle,
}

impl AppSettings {
    pub fn refresh_cli_installation(&self) -> anyhow::Result<()> {
        let executable = cli_executable()?;
        self.observable.try_set_state(|mut state| {
            state.is_cli_installed = super::cli::is_installed(
                &std::path::Path::new("/usr/local/bin").join(&state.cli_command),
                &executable,
            )?;
            Ok(state)
        })
    }

    pub fn install_cli(&self) -> anyhow::Result<()> {
        let state = self.observable.get_state()?;
        let destination = std::path::Path::new("/usr/local/bin").join(&state.cli_command);
        super::cli::install(&cli_executable()?, &destination)?;
        self.refresh_cli_installation()
    }

    pub fn apply_all(&self) -> anyhow::Result<()> {
        let state = self.observable.get_state()?;
        self.apply_state(&state)
    }

    pub fn toggle_menu_bar_icon(&self) -> anyhow::Result<()> {
        self.update_state(|state| {
            state.show_menu_bar_icon = !state.show_menu_bar_icon;
        })
    }

    pub fn toggle_dock_icon(&self) -> anyhow::Result<()> {
        self.update_state(|state| {
            state.show_dock_icon = !state.show_dock_icon;
        })
    }

    pub fn toggle_hide_hint_panel_by_default(&self) -> anyhow::Result<()> {
        self.update_state(|state| {
            state.is_chord_panel_hidden_by_default = !state.is_chord_panel_hidden_by_default;
        })
    }

    pub fn load_settings_sidebar(&self) -> anyhow::Result<()> {
        let collapsed = Self::read_bool_setting(
            &self.handle,
            SETTINGS_SIDEBAR_COLLAPSED_KEY,
            AppSettingsState::default().is_settings_sidebar_collapsed,
        )?;
        self.observable.set_state(|state| AppSettingsState {
            is_settings_sidebar_collapsed: collapsed,
            ..state
        })
    }

    /// Writes only its own key: `update_state` would re-apply the Dock and menu bar on every click.
    pub fn toggle_settings_sidebar(&self) -> anyhow::Result<()> {
        self.observable.try_set_state(|mut state| {
            state.is_settings_sidebar_collapsed = !state.is_settings_sidebar_collapsed;
            let store = self
                .handle
                .store(APP_STATE_STORE_PATH)
                .context("failed to open app state store")?;
            store.set(
                SETTINGS_SIDEBAR_COLLAPSED_KEY,
                state.is_settings_sidebar_collapsed,
            );
            store.save().context("failed to save app state store")?;
            Ok(state)
        })
    }

    /// Reads whether first-run setup is done. The first launch that finds nothing stored decides,
    /// and stores it: repositories exist only on an install from before onboarding, and by the
    /// next launch a new install has them too, whether or not setup was finished.
    pub fn load_onboarding(&self, has_existing_install: bool) -> anyhow::Result<()> {
        let store = self
            .handle
            .store(APP_STATE_STORE_PATH)
            .context("failed to open app state store")?;
        let complete = match store.get(ONBOARDING_COMPLETE_KEY) {
            Some(value) => serde_json::from_value(value)
                .with_context(|| format!("failed to parse `{ONBOARDING_COMPLETE_KEY}`"))?,
            None => {
                store.set(ONBOARDING_COMPLETE_KEY, has_existing_install);
                store.save().context("failed to save app state store")?;
                has_existing_install
            }
        };
        self.observable.set_state(|state| AppSettingsState {
            is_onboarding_complete: complete,
            ..state
        })
    }

    pub fn is_onboarding_complete(&self) -> bool {
        self.observable
            .get_state()
            .is_ok_and(|state| state.is_onboarding_complete)
    }

    pub fn set_onboarding_complete(&self, complete: bool) -> anyhow::Result<()> {
        self.update_state(|state| {
            state.is_onboarding_complete = complete;
        })
    }

    fn update_state<F>(&self, update: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut AppSettingsState),
    {
        self.observable.try_set_state(|prev| {
            let mut state = prev;
            update(&mut state);
            self.save_state(&state)?;
            self.apply_state(&state)?;
            Ok(state)
        })?;
        Ok(())
    }

    fn apply_state(&self, state: &AppSettingsState) -> anyhow::Result<()> {
        self.apply_menu_bar_icon_visibility(state.show_menu_bar_icon)?;
        self.apply_dock_icon_visibility(state.show_dock_icon)?;
        // self.apply_hint_panel_visibility(!state.is_chord_panel_hidden_by_default)?;
        Ok(())
    }

    fn apply_menu_bar_icon_visibility(&self, visible: bool) -> anyhow::Result<()> {
        if let Some(tray) = self.handle.tray_by_id(TRAY_ID) {
            tray.set_visible(visible)?;
        }

        Ok(())
    }

    fn apply_dock_icon_visibility(&self, visible: bool) -> anyhow::Result<()> {
        #[cfg(target_os = "macos")]
        self.handle.set_dock_visibility(visible)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn load_state(handle: &AppHandle) -> anyhow::Result<AppSettingsState> {
        let defaults = AppSettingsState::default();

        Ok(AppSettingsState {
            cli_command: defaults.cli_command,
            is_cli_installed: defaults.is_cli_installed,
            bundle_ids_needing_relaunch: defaults.bundle_ids_needing_relaunch,
            show_menu_bar_icon: Self::read_bool_setting(
                handle,
                SHOW_MENU_BAR_ICON_KEY,
                defaults.show_menu_bar_icon,
            )?,
            show_dock_icon: Self::read_bool_setting(
                handle,
                SHOW_DOCK_ICON_KEY,
                defaults.show_dock_icon,
            )?,
            is_chord_panel_hidden_by_default: Self::read_bool_setting(
                handle,
                HIDE_HINT_PANEL_BY_DEFAULT_KEY,
                defaults.is_chord_panel_hidden_by_default,
            )?,
            is_onboarding_complete: Self::read_bool_setting(
                handle,
                ONBOARDING_COMPLETE_KEY,
                defaults.is_onboarding_complete,
            )?,
            is_settings_sidebar_collapsed: Self::read_bool_setting(
                handle,
                SETTINGS_SIDEBAR_COLLAPSED_KEY,
                defaults.is_settings_sidebar_collapsed,
            )?,
        })
    }

    fn read_bool_setting(handle: &AppHandle, key: &str, default: bool) -> anyhow::Result<bool> {
        let store = handle
            .store(APP_STATE_STORE_PATH)
            .context("failed to open app state store")?;

        match store.get(key) {
            Some(value) => serde_json::from_value::<bool>(value)
                .with_context(|| format!("failed to parse app setting `{key}`")),
            None => Ok(default),
        }
    }

    fn save_state(&self, state: &AppSettingsState) -> anyhow::Result<()> {
        let store = self
            .handle
            .store(APP_STATE_STORE_PATH)
            .context("failed to open app state store")?;

        store.set(SHOW_MENU_BAR_ICON_KEY, state.show_menu_bar_icon);
        store.set(SHOW_DOCK_ICON_KEY, state.show_dock_icon);
        store.set(
            HIDE_HINT_PANEL_BY_DEFAULT_KEY,
            state.is_chord_panel_hidden_by_default,
        );
        store.set(ONBOARDING_COMPLETE_KEY, state.is_onboarding_complete);
        store.save().context("failed to save app state store")?;

        Ok(())
    }
}

fn cli_executable() -> anyhow::Result<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        crate::dev_runtime::cli_executable()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(std::env::current_exe()?)
    }
}

const SHOW_MENU_BAR_ICON_KEY: &str = "showMenuBarIcon";
const SHOW_DOCK_ICON_KEY: &str = "showDockIcon";
// Keeps the Hint Panel's former name: this key is already saved on existing installs.
const HIDE_HINT_PANEL_BY_DEFAULT_KEY: &str = "hideGuideByDefault";
const ONBOARDING_COMPLETE_KEY: &str = "onboardingComplete";
const SETTINGS_SIDEBAR_COLLAPSED_KEY: &str = "settingsSidebarCollapsed";
