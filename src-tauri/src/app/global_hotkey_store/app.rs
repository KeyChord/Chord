use tauri::AppHandle;
use crate::app::AppSingleton;
use crate::app::global_hotkey_store::global_hotkey_store::GlobalHotkeyStore;
use anyhow::Result;

impl<T> AppSingleton<T> for GlobalHotkeyStore {
    fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    fn init(&self, _: ()) -> Result<()> {
        Ok(())
    }
}