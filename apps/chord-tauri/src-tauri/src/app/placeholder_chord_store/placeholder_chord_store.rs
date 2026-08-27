use nject::injectable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

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

#[injectable]
#[derive(Clone)]
pub struct PlaceholderChordStore {
    pub handle: AppHandle,
}

impl PlaceholderChordStore {
    pub fn store(&self) -> anyhow::Result<Arc<Store<Wry>>> {
        Ok(self.handle.store(PLACEHOLDER_CHORDS_STORE_PATH)?)
    }

    pub fn entries(
        &self,
    ) -> anyhow::Result<HashMap<PlaceholderChordStoreKey, PlaceholderChordStoreEntry>> {
        Ok(self
            .store()?
            .entries()
            .into_iter()
            .filter_map(|(key, value)| {
                let parsed_key = serde_json::from_str::<PlaceholderChordStoreKey>(&key).ok()?;
                let parsed_value =
                    serde_json::from_value::<PlaceholderChordStoreEntry>(value.clone()).ok()?;
                Some((parsed_key, parsed_value))
            })
            .collect())
    }

    #[allow(dead_code)]
    pub fn entry(
        &self,
        key: &PlaceholderChordStoreKey,
    ) -> anyhow::Result<Option<PlaceholderChordStoreEntry>> {
        Ok(self.entries()?.get(key).cloned())
    }

    fn serialize_key(key: &PlaceholderChordStoreKey) -> String {
        serde_json::to_string(key).expect("placeholder chord store key should serialize")
    }

    fn save(&self) -> anyhow::Result<()> {
        self.store()?.save()?;
        Ok(())
    }

    pub fn set(
        &self,
        key: PlaceholderChordStoreKey,
        entry: PlaceholderChordStoreEntry,
    ) -> anyhow::Result<()> {
        let serialized_key = Self::serialize_key(&key);
        let value =
            serde_json::to_value(entry).expect("placeholder chord store entry should serialize");
        self.store()?.set(serialized_key, value);
        self.save()?;
        Ok(())
    }

    pub fn remove(&self, key: &PlaceholderChordStoreKey) -> anyhow::Result<()> {
        let serialized_key = Self::serialize_key(key);
        self.store()?.delete(serialized_key);
        self.save()?;
        Ok(())
    }
}

pub fn normalize_placeholder_sequence(sequence: &str) -> anyhow::Result<String> {
    let normalized = sequence.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        anyhow::bail!("placeholder sequence cannot be empty");
    }

    if !normalized.chars().all(|ch| ch.is_ascii_lowercase()) {
        anyhow::bail!("placeholder sequence must only contain letters a-z");
    }

    Ok(normalized)
}
