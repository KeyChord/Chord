use crate::observables::{AppSettingsObservable, AppSettingsState, ChorderObservable, Observable};
use crate::tauri_app::startup::APP_STATE_STORE_PATH;
use crate::tauri_app::tray::TRAY_ID;
use anyhow::{Context, Result};
use std::sync::Arc;
use tauri::{Manager, WebviewUrl, WebviewWindow};
use crate::app::SafeAppHandle;

pub struct AppSettings {
    pub ui: SettingsUi,
    observable: Arc<AppSettingsObservable>,
    handle: SafeAppHandle,
}

pub struct SettingsUi {
    pub handle: SafeAppHandle,
}

impl SettingsUi {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        Ok(Self { handle })
    }

    pub fn get_or_create_window(&self) -> Result<WebviewWindow> {
        if let Some(window) = self.handle.get_webview_window("settings") {
            return Ok(window);
        }

        // 🔥 otherwise create it
        let window = self
            .handle
            .new_webview_window_builder("settings", WebviewUrl::App("index.html".into()))?
            .title("Settings")
            .inner_size(920.0, 760.0)
            .min_inner_size(760.0, 620.0)
            .visible(false)
            .focused(false)
            .resizable(true)
            .center()
            .build()?;

        Ok(window)
    }

    pub fn open(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        Ok(())
    }

    pub fn open_inspector(&self) -> Result<()>{
        let window = self.get_or_create_window()?;
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        #[cfg(debug_assertions)]
        window.open_devtools();
        Ok(())
    }

    pub fn hide(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        window.hide()?;
        Ok(())
    }
}

impl AppSettings {
    pub fn new(handle: SafeAppHandle, observable: Arc<AppSettingsObservable>) -> Result<Self> {
        let ui = SettingsUi::new(handle.clone())?;
        observable.set_state(Self::load_state(&handle)?)?;
        Ok(Self {
            ui,
            observable,
            handle,
        })
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
        self.apply_guide_visibility(!state.hide_guide_by_default)?;
        Ok(())
    }

    fn apply_menu_bar_icon_visibility(&self, visible: bool) -> Result<()> {
        if let Some(tray) = self.handle.handle().tray_by_id(TRAY_ID) {
            tray.set_visible(visible)?;
        }

        Ok(())
    }

    fn apply_dock_icon_visibility(&self, visible: bool) -> Result<()> {
        #[cfg(target_os = "macos")]
        self.handle.handle().set_dock_visibility(visible)?;

        Ok(())
    }

    // UNSAFE
    fn apply_guide_visibility(&self, visible: bool) -> Result<()> {
        let handle = self.handle.try_handle()?;
        let chorder = handle.state::<Arc<ChorderObservable>>();
        let current_state = chorder.get_state()?;
        chorder.set_state(crate::observables::ChorderState {
            is_indicator_visible: visible,
            ..current_state.as_ref().clone()
        })?;

        Ok(())
    }

    fn load_state(handle: &SafeAppHandle) -> Result<AppSettingsState> {
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

    fn read_bool_setting(handle: &SafeAppHandle, key: &str, default: bool) -> Result<bool> {
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
