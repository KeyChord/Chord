use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Wry;
use tauri_plugin_store::Store;
use crate::app::SafeAppHandle;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalHotkeyStoreEntry {
    pub bundle_id: String,
    pub hotkey_id: String,
}

#[derive(Clone)]
pub struct GlobalHotkeyStore {
    pub store: Arc<Store<Wry>>,
}

impl GlobalHotkeyStore {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let store = handle.store("global-hotkeys.json")?;
        Ok(Self { store })
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

    pub fn entry(&self, shortcut: &str) -> Option<GlobalHotkeyStoreEntry> {
        self.entries().get(shortcut).cloned()
    }

    fn save(&self) -> Result<()> {
        self.store.save()?;
        Ok(())
    }

    pub fn set(&self, shortcut: &str, entry: GlobalHotkeyStoreEntry) -> Result<()> {
        let value = serde_json::to_value(entry).unwrap();
        self.store.set(shortcut, value);
        self.save()?;
        Ok(())
    }

    pub fn remove(&self, shortcut: &str) -> Result<()> {
        self.store.delete(shortcut);
        self.save()?;
        Ok(())
    }

    pub fn update_shortcut(&self, old_shortcut: &str, new_shortcut: &str) -> Result<()> {
        let Some(entry) = self.entry(old_shortcut) else {
            anyhow::bail!("global shortcut mapping not found");
        };

        if old_shortcut != new_shortcut && self.entry(new_shortcut).is_some() {
            anyhow::bail!("shortcut is already assigned");
        }

        self.store.delete(old_shortcut);
        let value = serde_json::to_value(entry).unwrap();
        self.store.set(new_shortcut, value);
        self.save()?;
        Ok(())
    }
}
