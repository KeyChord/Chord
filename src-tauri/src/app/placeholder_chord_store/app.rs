use crate::app::AppSingleton;
use crate::app::placeholder_chord_store::placeholder_chord_store::PlaceholderChordStore;
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct PlaceholderChordStoreProvider {
    #[provide(AppHandle, |h| h.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for PlaceholderChordStore {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
