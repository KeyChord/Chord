use crate::state::{ChordPackageStoreObservable, ChordPackageStoreState, Observable};
use anyhow::Result;
use nject::injectable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};
use typeshare::typeshare;

#[injectable]
#[derive(Clone)]
pub struct ChordPackageStore {
    handle: AppHandle,
    observable: ChordPackageStoreObservable,
}

#[typeshare]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackageStoreEntry {
    pub priority: u32,
}

impl ChordPackageStore {
    pub fn store(&self) -> Result<Arc<Store<Wry>>> {
        let store = self.handle.store("chord-package-registry.json")?;
        Ok(store)
    }

    pub fn entries(&self) -> Result<HashMap<String, ChordPackageStoreEntry>> {
        Ok(self
            .store()?
            .entries()
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<ChordPackageStoreEntry>(v.clone())
                    .ok()
                    .map(|entry| (k.to_string(), entry))
            })
            .collect())
    }

    pub fn entry(&self, shortcut: &str) -> Result<Option<ChordPackageStoreEntry>> {
        Ok(self.entries()?.get(shortcut).cloned())
    }

    fn save(&self) -> Result<()> {
        self.store()?.save()?;
        self.observable.try_set_state(|state| Ok(ChordPackageStoreState {
            entries: self.entries()?,
        }))?;
        Ok(())
    }

    pub fn set(&self, shortcut: &str, entry: ChordPackageStoreEntry) -> Result<()> {
        let value = serde_json::to_value(entry)?;
        self.store()?.set(shortcut, value);
        self.save()?;
        Ok(())
    }

    pub fn remove(&self, shortcut: &str) -> Result<()> {
        self.store()?.delete(shortcut);
        self.save()?;
        Ok(())
    }
}
