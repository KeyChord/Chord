use crate::constants::{CHORD_WINDOW_LABEL, SETTINGS_WINDOW_LABEL};
use anyhow::Result;
use tauri::{AppHandle, Manager, WebviewWindow};

pub fn get_settings_window(handle: AppHandle) -> Result<WebviewWindow> {
    handle
        .get_webview_window(SETTINGS_WINDOW_LABEL)
        .ok_or(anyhow::anyhow!("settings window not found"))
}

pub fn get_chords_window(handle: AppHandle) -> Result<WebviewWindow> {
    handle
        .get_webview_window(CHORD_WINDOW_LABEL)
        .ok_or(anyhow::anyhow!("chord window not found"))
}

pub fn configure_settings_window(handle: AppHandle) -> Result<()> {
    let _window = get_settings_window(handle)?;
    Ok(())
}

pub fn show_settings_window(handle: AppHandle) -> Result<()> {
    let window = get_settings_window(handle)?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}

pub fn open_settings_inspector(handle: AppHandle) -> Result<()> {
    let window = get_settings_window(handle)?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    #[cfg(debug_assertions)]
    window.open_devtools();
    Ok(())
}

pub fn open_chords_inspector(handle: AppHandle) -> Result<()> {
    let window = get_chords_window(handle)?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    #[cfg(debug_assertions)]
    window.open_devtools();
    Ok(())
}

pub fn hide_settings_window(handle: AppHandle) -> Result<()> {
    let window = get_settings_window(handle)?;
    window.hide()?;
    Ok(())
}
