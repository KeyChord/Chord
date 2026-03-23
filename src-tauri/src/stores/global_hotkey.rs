use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Runtime, Wry};
use tauri_plugin_store::Store;
use crate::AppContext;
use crate::feature::app_handle_ext::AppHandleExt;
use crate::feature::{AppChorder, AppFrontmost, AppPermissions, AppSettings};
use crate::tauri_app::git::ChordPackageRegistry;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalHotkeyStoreEntry {
    pub bundle_id: String,
    pub hotkey_id: String,
}

pub struct GlobalHotkeyStore<R: Runtime> {
    pub store: Arc<Store<R>>,
}

impl<R: Runtime> Clone for GlobalHotkeyStore<R> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone()
        }
    }
}

impl<R: Runtime> GlobalHotkeyStore<R> {
    pub fn new(store: Arc<Store<R>>) -> Self {
        Self { store }
    }

    pub fn entries(&self) -> HashMap<String, GlobalHotkeyStoreEntry> {
        // We clone it to avoid deadlocks (since .entries() calls a lock)
        self.store
            .entries()
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<GlobalHotkeyStoreEntry>(v.clone())
                    .ok()
                    .map(|entry| (k.to_string(), entry))
            })
            .collect()
    }

    pub fn set(&self, shortcut: &str, entry: GlobalHotkeyStoreEntry) {
        let value = serde_json::to_value(entry).unwrap();
        self.store.set(shortcut, value);
    }

    pub fn remove(&self, shortcut: &str) {
        self.store.delete(shortcut);
    }
}


