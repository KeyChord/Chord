use ordermap::OrderMap;
use parking_lot::RwLock;
use tauri::AppHandle;
use crate::app::chord_package_manager::ChordPackageManager;
use crate::app::chord_package_registry::ChordPackageRegistry;
use crate::app::state::AppSingleton;
use crate::state::{ChordPackageManagerObservable, Observable};
use anyhow::Result;

impl AppSingleton<ChordPackageManagerObservable> for ChordPackageManager {
    fn new(handle: AppHandle) -> Self {
        Self {
            packages: RwLock::new(OrderMap::new()),
            registry: ChordPackageRegistry::new(handle.clone()),
            observable: ChordPackageManagerObservable::uninitialized(),
            handle,
        }
    }

    fn init(&self, observable: ChordPackageManagerObservable) -> Result<()> {
        self.observable.init(observable);
        Ok(())
    }
}
