use crate::feature::SafeAppHandle;
use anyhow::Result;
use tauri::{Emitter, WebviewUrl, WebviewWindow};

pub struct AppSettings {
    pub ui: SettingsUi,
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
            .resizable(true)
            .center()
            .build()?;

        Ok(window)
    }
}

impl AppSettings {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let ui = SettingsUi::new(handle.clone())?;
        Ok(Self { ui })
    }
}
