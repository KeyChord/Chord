use crate::app::chord_package_store::ChordPackageStore;
use crate::app::state::AppSingleton;
use crate::state::{ChordPackageStoreObservable, Observable};
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct ChordPackageStoreProvider {
    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for ChordPackageStore {
    fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
