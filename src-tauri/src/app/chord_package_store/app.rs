use tauri::AppHandle;
use crate::app::chord_package_store::ChordPackageStore;
use crate::app::state::AppSingleton;
use crate::state::{ChordPackageStoreObservable, Observable};

impl AppSingleton<ChordPackageStoreObservable> for ChordPackageStore {
    fn new(handle: AppHandle) -> Self {
        Self {
            handle,
            observable: ChordPackageStoreObservable::uninitialized(),
        }
    }

    fn init(&self, observable: ChordPackageStoreObservable) -> anyhow::Result<()> {
        self.observable.init(observable);
        Ok(())
    }
}
