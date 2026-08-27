use crate::app::AppSingleton;
use crate::app::global_hotkey_store::global_hotkey_store::GlobalHotkeyStore;
use crate::state::FrontmostObservable;
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct GlobalHotkeyStoreProvider {
    #[provide(AppHandle, |h| h.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for GlobalHotkeyStore {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
