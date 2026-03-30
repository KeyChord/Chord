
use anyhow::Result;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use crate::app::state::StateSingleton;

pub struct SettingsUi {
    pub handle: AppHandle,
}

impl SettingsUi {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
    
    pub fn get_or_create_window(&self) -> Result<WebviewWindow> {
        if let Some(window) = self.handle.get_webview_window("settings") {
            return Ok(window);
        }

        // 🔥 otherwise create it
        let window =
            WebviewWindowBuilder::new(&self.handle, "settings", WebviewUrl::App("index.html".into()))
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

    #[allow(dead_code)]
    pub fn open_inspector(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        #[cfg(debug_assertions)]
        window.open_devtools();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn hide(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        window.hide()?;
        Ok(())
    }
}
