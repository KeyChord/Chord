use nject::injectable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

pub const PLACEHOLDER_CHORDS_STORE_PATH: &str = "placeholder-chords.json";

/// Saved values by chords file pathslug and then placeholder token.
pub type FilePlaceholderAssignments = HashMap<String, HashMap<String, String>>;

/// The user's value for one placeholder in one chords file. Keyed by package name and pathslug,
/// not by location, so it carries over when the same package loads from another source (a local
/// folder, a monorepo or its GitHub mirror). Entries in an older key shape are ignored.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderChordStoreKey {
    pub package: String,
    /// The chords file's pathslug, such as `chords/macos.toml`.
    pub file: String,
    /// The complete placeholder token. Older entries store a bare name.
    pub placeholder: String,
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

    pub fn assigned_by_package(
        &self,
    ) -> anyhow::Result<HashMap<String, FilePlaceholderAssignments>> {
        let mut packages = HashMap::<String, FilePlaceholderAssignments>::new();
        for (key, entry) in self.entries()? {
            packages
                .entry(key.package)
                .or_default()
                .entry(key.file)
                .or_default()
                .insert(key.placeholder, entry.sequence);
        }
        Ok(packages)
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

    pub fn remove(
        &self,
        key: PlaceholderChordStoreKey,
        legacy_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let store = self.store()?;
        let serialized_key = Self::serialize_key(&key);
        if let Some(legacy_name) = legacy_name {
            let legacy_key = PlaceholderChordStoreKey {
                placeholder: legacy_name.to_owned(),
                ..key
            };
            store.delete(Self::serialize_key(&legacy_key));
        }
        store.delete(serialized_key);
        store.save()?;
        Ok(())
    }
}

/// Trims surrounding whitespace and checks the value the same way the chords file parser does.
pub fn normalize_placeholder_sequence(sequence: &str) -> anyhow::Result<String> {
    let normalized = sequence.trim();
    crate::models::validate_placeholder_value(normalized)?;
    Ok(normalized.to_owned())
}
