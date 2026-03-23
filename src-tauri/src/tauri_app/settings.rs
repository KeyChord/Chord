use crate::feature::app_handle_ext::AppHandleExt;
use anyhow::Result;
use tauri::{AppHandle, Manager, WebviewWindow};
use crate::AppContext;
use crate::feature::{AppChorder, AppSettings};

pub fn show_settings_window(handle: AppHandle) -> Result<()> {
    let settings = handle.app_settings();
    let window = &settings.ui.window;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}

pub fn hide_settings_window(handle: AppHandle) -> Result<()> {
    let settings = handle.app_settings();
    let window = &settings.ui.window;
    window.hide()?;
    Ok(())
}

pub fn open_settings_inspector(handle: AppHandle) -> Result<()> {
    let settings = handle.app_settings();
    let window = &settings.ui.window;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    #[cfg(debug_assertions)]
    window.open_devtools();
    Ok(())
}

pub fn open_chords_inspector(handle: AppHandle) -> Result<()> {
    let chorder = handle.app_chorder();
    let window = &chorder.ui.window;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    #[cfg(debug_assertions)]
    window.open_devtools();
    Ok(())
}

