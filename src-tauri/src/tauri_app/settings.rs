use anyhow::Result;
use tauri::{AppHandle, Manager, WebviewWindow};
use crate::AppContext;

pub fn show_settings_window(handle: AppHandle) -> Result<()> {
    let context = handle.state::<AppContext>();
    let window = &context.settings.window;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}

pub fn open_settings_inspector(handle: AppHandle) -> Result<()> {
    let context = handle.state::<AppContext>();
    let window = &context.settings.window;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    #[cfg(debug_assertions)]
    window.open_devtools();
    Ok(())
}

pub fn open_chords_inspector(handle: AppHandle) -> Result<()> {
    let context = handle.state::<AppContext>();
    let window = &context.chorder.ui.window;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    #[cfg(debug_assertions)]
    window.open_devtools();
    Ok(())
}

pub fn hide_settings_window(handle: AppHandle) -> Result<()> {
    let context = handle.state::<AppContext>();
    let window = &context.settings.window;
    window.hide()?;
    Ok(())
}
