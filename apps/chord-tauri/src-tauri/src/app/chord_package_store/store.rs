use crate::state::{ChordPackageStoreObservable, ChordPackageStoreState, Observable};
use anyhow::Result;
use nject::injectable;
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

const ORDER_KEY: &str = "order";

/// The user's ranking of installed and built-in packages, by package name, highest first.
/// Chord's protected core package and personal packages are not part of it (see
/// `chord_package_manager::rank_packages`).
#[injectable]
pub struct ChordPackageStore {
    handle: AppHandle,
    observable: ChordPackageStoreObservable,
}

impl ChordPackageStore {
    fn store(&self) -> Result<Arc<Store<Wry>>> {
        Ok(self.handle.store("chord-package-order.json")?)
    }

    /// Names in the saved order, including packages that are not loaded right now (an
    /// unmounted local folder keeps its place for when it comes back).
    pub fn order(&self) -> Result<Vec<String>> {
        Ok(self
            .store()?
            .get(ORDER_KEY)
            .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
            .unwrap_or_default())
    }

    pub fn set_order(&self, order: Vec<String>) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        let order: Vec<String> = order
            .into_iter()
            .filter(|name| seen.insert(name.clone()))
            .collect();
        if order == self.order()? {
            return Ok(());
        }
        let store = self.store()?;
        store.set(ORDER_KEY, serde_json::to_value(&order)?);
        store.save()?;
        self.observable
            .set_state(|_| ChordPackageStoreState { order })?;
        Ok(())
    }

    pub(super) fn publish(&self) -> Result<()> {
        let order = self.order()?;
        self.observable
            .set_state(|_| ChordPackageStoreState { order })?;
        Ok(())
    }
}
