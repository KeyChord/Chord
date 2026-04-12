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
    handle: AppHandle,
}

impl AppSettings {
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

    pub fn toggle_hide_guide_by_default(&self) -> anyhow::Result<()> {
        self.update_state(|state| {
            state.is_chord_panel_hidden_by_default = !state.is_chord_panel_hidden_by_default;
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
        // self.apply_guide_visibility(!state.hide_guide_by_default)?;
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
                HIDE_GUIDE_BY_DEFAULT_KEY,
                defaults.is_chord_panel_hidden_by_default,
            )?,
        })
    }

    #[allow(dead_code)]
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
        store.set(HIDE_GUIDE_BY_DEFAULT_KEY, state.is_chord_panel_hidden_by_default);
        store.save().context("failed to save app state store")?;

        Ok(())
    }
}

const SHOW_MENU_BAR_ICON_KEY: &str = "showMenuBarIcon";
const SHOW_DOCK_ICON_KEY: &str = "showDockIcon";
const HIDE_GUIDE_BY_DEFAULT_KEY: &str = "hideGuideByDefault";
