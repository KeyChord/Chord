#[allow(unused_imports)]
use crate::api::{AppError, AppResult};
use crate::app::SafeAppHandle;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Wry;
use tauri_plugin_store::Store;

pub const PLACEHOLDER_CHORDS_STORE_PATH: &str = "placeholder-chords.json";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct PlaceholderChordStoreKey {
    pub file_path: String,
    pub sequence_template: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaceholderChordStoreEntry {
    pub sequence: String,
}

#[derive(Clone)]
pub struct PlaceholderChordStore {
    pub store: Arc<Store<Wry>>,
}

impl PlaceholderChordStore {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let store = handle.store(PLACEHOLDER_CHORDS_STORE_PATH)?;
        Ok(Self { store })
    }

    pub fn entries(&self) -> HashMap<PlaceholderChordStoreKey, PlaceholderChordStoreEntry> {
        self.store
            .entries()
            .into_iter()
            .filter_map(|(key, value)| {
                let parsed_key = serde_json::from_str::<PlaceholderChordStoreKey>(&key).ok()?;
                let parsed_value =
                    serde_json::from_value::<PlaceholderChordStoreEntry>(value.clone()).ok()?;
                Some((parsed_key, parsed_value))
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn entry(&self, key: &PlaceholderChordStoreKey) -> Option<PlaceholderChordStoreEntry> {
        self.entries().get(key).cloned()
    }

    fn serialize_key(key: &PlaceholderChordStoreKey) -> String {
        serde_json::to_string(key).expect("placeholder chord store key should serialize")
    }

    fn save(&self) -> Result<()> {
        self.store.save()?;
        Ok(())
    }

    pub fn set(
        &self,
        key: PlaceholderChordStoreKey,
        entry: PlaceholderChordStoreEntry,
    ) -> Result<()> {
        let serialized_key = Self::serialize_key(&key);
        let value =
            serde_json::to_value(entry).expect("placeholder chord store entry should serialize");
        self.store.set(serialized_key, value);
        self.save()?;
        Ok(())
    }

    pub fn remove(&self, key: &PlaceholderChordStoreKey) -> Result<()> {
        let serialized_key = Self::serialize_key(key);
        self.store.delete(serialized_key);
        self.save()?;
        Ok(())
    }
}

pub fn normalize_placeholder_sequence(sequence: &str) -> Result<String> {
    let normalized = sequence.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        anyhow::bail!("placeholder sequence cannot be empty");
    }

    if !normalized.chars().all(|ch| ch.is_ascii_lowercase()) {
        anyhow::bail!("placeholder sequence must only contain letters a-z");
    }

    Ok(normalized)
}
