use anyhow::Result;
use tauri::{AppHandle, Runtime};
use crate::app::AppHandleExt;

pub fn open_chords_inspector<R: Runtime>(handle: AppHandle<R>) -> Result<()> {
    let chorder = handle.app_chorder();
    let window = &chorder.ui.window;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    #[cfg(debug_assertions)]
    window.open_devtools();
    Ok(())
}
