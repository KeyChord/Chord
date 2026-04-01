use self::ui::SettingsUi;
use crate::app::state::StateSingleton;
use crate::observables::{AppSettingsObservable, AppSettingsState, Observable};
use crate::tauri_app::startup::APP_STATE_STORE_PATH;
use crate::tauri_app::tray::TRAY_ID;
use anyhow::{Context, Result};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

mod ui;

pub struct AppSettings {
    pub ui: SettingsUi,
    observable: AppSettingsObservable,
    handle: AppHandle,
}
impl StateSingleton for AppSettings {
    fn new(handle: AppHandle) -> Self {
        Self {
            ui: SettingsUi::new(handle.clone()),
            observable: AppSettingsObservable::placeholder(),
            handle,
        }
    }
}

impl AppSettings {
    pub fn init(&self, observable: AppSettingsObservable) -> Result<()> {
        self.observable.init(observable);
        Ok(())
    }

    pub fn apply_all(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        self.apply_state(state.as_ref())
    }

    pub fn toggle_menu_bar_icon(&self) -> Result<()> {
        self.update_state(|state| {
            state.show_menu_bar_icon = !state.show_menu_bar_icon;
        })
    }

    pub fn toggle_dock_icon(&self) -> Result<()> {
        self.update_state(|state| {
            state.show_dock_icon = !state.show_dock_icon;
        })
    }

    pub fn toggle_hide_guide_by_default(&self) -> Result<()> {
        self.update_state(|state| {
            state.hide_guide_by_default = !state.hide_guide_by_default;
        })
    }

    fn update_state<F>(&self, update: F) -> Result<()>
    where
        F: FnOnce(&mut AppSettingsState),
    {
        let mut next_state = self.observable.get_state()?.as_ref().clone();
        update(&mut next_state);
        self.save_state(&next_state)?;
        self.observable.set_state(next_state.clone())?;
        self.apply_state(&next_state)?;
        Ok(())
    }

    fn apply_state(&self, state: &AppSettingsState) -> Result<()> {
        self.apply_menu_bar_icon_visibility(state.show_menu_bar_icon)?;
        self.apply_dock_icon_visibility(state.show_dock_icon)?;
        // self.apply_guide_visibility(!state.hide_guide_by_default)?;
        Ok(())
    }

    fn apply_menu_bar_icon_visibility(&self, visible: bool) -> Result<()> {
        if let Some(tray) = self.handle.tray_by_id(TRAY_ID) {
            tray.set_visible(visible)?;
        }

        Ok(())
    }

    fn apply_dock_icon_visibility(&self, visible: bool) -> Result<()> {
        #[cfg(target_os = "macos")]
        self.handle.set_dock_visibility(visible)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn load_state(handle: &AppHandle) -> Result<AppSettingsState> {
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
            hide_guide_by_default: Self::read_bool_setting(
                handle,
                HIDE_GUIDE_BY_DEFAULT_KEY,
                defaults.hide_guide_by_default,
            )?,
        })
    }

    #[allow(dead_code)]
    fn read_bool_setting(handle: &AppHandle, key: &str, default: bool) -> Result<bool> {
        let store = handle
            .store(APP_STATE_STORE_PATH)
            .context("failed to open app state store")?;

        match store.get(key) {
            Some(value) => serde_json::from_value::<bool>(value)
                .with_context(|| format!("failed to parse app setting `{key}`")),
            None => Ok(default),
        }
    }

    fn save_state(&self, state: &AppSettingsState) -> Result<()> {
        let store = self
            .handle
            .store(APP_STATE_STORE_PATH)
            .context("failed to open app state store")?;

        store.set(SHOW_MENU_BAR_ICON_KEY, state.show_menu_bar_icon);
        store.set(SHOW_DOCK_ICON_KEY, state.show_dock_icon);
        store.set(HIDE_GUIDE_BY_DEFAULT_KEY, state.hide_guide_by_default);
        store.save().context("failed to save app state store")?;

        Ok(())
    }
}

const SHOW_MENU_BAR_ICON_KEY: &str = "showMenuBarIcon";
const SHOW_DOCK_ICON_KEY: &str = "showDockIcon";
const HIDE_GUIDE_BY_DEFAULT_KEY: &str = "hideGuideByDefault";
