use anyhow::Result;
use nject::injectable;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use tauri::{
    LogicalPosition, TitleBarStyle,
    window::{Effect, EffectState, EffectsBuilder},
};

#[injectable]
pub struct SettingsUi {
    pub handle: AppHandle,
}

impl SettingsUi {
    pub fn get_or_create_window(&self) -> Result<WebviewWindow> {
        if let Some(window) = self.handle.get_webview_window("settings") {
            return Ok(window);
        }

        // 🔥 otherwise create it
        let window_builder = WebviewWindowBuilder::new(
            &self.handle,
            "settings",
            WebviewUrl::App("index.html".into()),
        )
        .title("Chord Settings")
        .inner_size(920.0, 760.0)
        .min_inner_size(760.0, 620.0)
        .visible(false)
        .focused(false)
        .resizable(true)
        .center();

        #[cfg(target_os = "macos")]
        let window_builder = window_builder
            .transparent(true)
            .title_bar_style(TitleBarStyle::Overlay)
            .hidden_title(true)
            .traffic_light_position(LogicalPosition::new(18.0, 24.0))
            .effects(
                EffectsBuilder::new()
                    .effect(Effect::Sidebar)
                    .state(EffectState::FollowsWindowActiveState)
                    .build(),
            );

        let window = window_builder.build()?;

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
