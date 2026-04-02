use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};
use crate::observables::{ChordPackageRegistryObservable, Observable};
use typeshare::typeshare;

#[typeshare]
#[derive(Clone)]
pub struct ChordPackageRegistryStore {
    handle: AppHandle,
    observable: ChordPackageRegistryObservable,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChordPackageRegistryEntry {
    pub priority: u32,
}

impl ChordPackageRegistryStore {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle, observable: ChordPackageRegistryObservable::placeholder() }
    }

    pub fn store(&self) -> Result<Arc<Store<Wry>>> {
        let store = self.handle.store("chord-package-registry.json")?;
        Ok(store)
    }

    pub fn entries(&self) -> Result<HashMap<String, ChordPackageRegistryEntry>> {
        Ok(self
            .store()?
            .entries()
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<ChordPackageRegistryEntry>(v.clone())
                    .ok()
                    .map(|entry| (k.to_string(), entry))
            })
            .collect())
    }

    pub fn entry(&self, shortcut: &str) -> Result<Option<ChordPackageRegistryEntry>> {
        Ok(self.entries()?.get(shortcut).cloned())
    }

    fn save(&self) -> Result<()> {
        self.store()?.save()?;
        Ok(())
    }

    pub fn set(&self, shortcut: &str, entry: ChordPackageRegistryEntry) -> Result<()> {
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
