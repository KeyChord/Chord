use crate::app::SafeAppHandle;
use crate::observables::{AppSettingsObservable, AppSettingsState, ChorderObservable, Observable};
use crate::tauri_app::startup::APP_STATE_STORE_PATH;
use crate::tauri_app::tray::TRAY_ID;
use anyhow::{Context, Result};
use std::sync::Arc;
use tauri::{Manager, WebviewUrl, WebviewWindow};

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

    pub fn open_inspector(&self) -> Result<()> {
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
