use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Runtime;
use tauri_plugin_store::Store;
use crate::observables::GitRepo;
use crate::stores::global_hotkey::GlobalHotkeyStore;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitReposStoreEntry {
    pub repo: GitRepo
}

#[derive(Clone)]
pub struct GitReposStore<R: Runtime> {
    pub store: Arc<Store<R>>,
}

impl<R: Runtime> Clone for GitReposStore<R> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone()
        }
    }
}

impl<R: Runtime> GitReposStore<R> {
    pub fn new(store: Arc<Store<R>>) -> Self {
        Self { store }
    }

    pub fn entries(&self) -> HashMap<String, GitReposStoreEntry> {
        // We clone it to avoid deadlocks (since .entries() calls a lock)
        self.store
            .entries()
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<GitReposStoreEntry>(v.clone())
                    .ok()
                    .map(|entry| (k.to_string(), entry))
            })
            .collect()
    }

    pub fn set(&self, shortcut: &str, entry: GitReposStoreEntry) {
        let value = serde_json::to_value(entry).unwrap();
        self.store.set(shortcut, value);
    }

    pub fn remove(&self, shortcut: &str) {
        self.store.delete(shortcut);
    }
}
