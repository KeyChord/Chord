use tauri::AppHandle;
use crate::app::AppSingleton;
use crate::app::placeholder_chord_store::placeholder_chord_store::PlaceholderChordStore;

impl AppSingleton<()> for PlaceholderChordStore {
    fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
    fn init(&self, _: ()) -> anyhow::Result<()> {
        Ok(())
    }
}

