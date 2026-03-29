mod loader;
mod parser;
mod file;
mod chords_file_parser;
mod chords_file_models;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use fast_radix_trie::StringRadixMap;
use walkdir::WalkDir;
use crate::app::chord_package::raw::ChordPackage;
use crate::app::chord_runner::runtime::ChordRuntime;
use crate::app::placeholder_chord_store::{PlaceholderChordStoreEntry, PlaceholderChordStoreKey};
use crate::app::SafeAppHandle;
use crate::input::Key;
use crate::observables::PlaceholderChordInfo;

struct ChordPackageManager {
    packages: HashMap<String, ChordPackage>,

    handle: SafeAppHandle,
}


impl ChordPackageManager {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle, packages: HashMap::new() }
    }


    pub async fn load_packages(&self, chord_packages: Vec<ChordPackage>) -> anyhow::Result<()> {
        let handle = self.handle.try_handle()?;
        for chord_package in &chord_packages {
            let js_files = chord_package.js_files.clone();
            let js_package_registry = handle.app_chord_js_package_registry();
            for (relpath, contents) in js_files {
                js_package_registry.load_module(contents).await?;
            }
        }

        let placeholder_entries = handle.app_placeholder_chord_store().entries();
        let (
            global_chords_to_runtime_key,
            app_runtime_map,
            runtime_bundle_ids,
            runtime_order,
            raw_files_map,
            mut placeholder_chords,
        ) = ChordRunnerRegistry::parse_packages(chord_packages, &placeholder_entries)?;
        placeholder_chords.sort_by(|left, right| {
            left.scope_kind
                .cmp(&right.scope_kind)
                .then(left.scope.cmp(&right.scope))
                .then(left.name.cmp(&right.name))
                .then(left.placeholder.cmp(&right.placeholder))
        });

        // Load the metadata
        // TODO: this hangs for some reason
        // let desktop_app_manager = handle.desktop_app_manager();
        // let bundle_ids: Vec<&str> = app_runtime_map.keys().map(|k| k.as_str()).collect();
        // desktop_app_manager.load_apps_metadata(&bundle_ids)?;

        // Set state before setting observable
        {
            let mut map = self.global_chords_to_runtime_key.lock().expect("poisoned");
            *map = global_chords_to_runtime_key;
        }

        let runtime_index = app_runtime_map
            .into_iter()
            .map(|(runtime_id, runtime)| (runtime_id, Arc::new(runtime)))
            .collect::<HashMap<_, _>>();
        let mut runtimes = HashMap::<String, Vec<Arc<ChordRuntime>>>::new();
        let mut seen_runtime_ids = HashSet::new();
        let mut deduped_runtime_order = runtime_order
            .into_iter()
            .rev()
            .filter(|runtime_id| seen_runtime_ids.insert(runtime_id.clone()))
            .collect::<Vec<_>>();
        deduped_runtime_order.reverse();
        for runtime_id in deduped_runtime_order {
            let Some(bundle_id) = runtime_bundle_ids.get(&runtime_id) else {
                continue;
            };
            let Some(runtime) = runtime_index.get(&runtime_id) else {
                continue;
            };
            runtimes
                .entry(bundle_id.clone())
                .or_default()
                .push(runtime.clone());
        }

        {
            let mut map = self.runtime_index.lock().expect("poisoned");
            *map = runtime_index;
        }

        {
            let mut map = self.runtimes.lock().expect("poisoned");
            *map = runtimes;
        }

        let raw_files_as_json_strings = raw_files_map
            .iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::to_string(v)?)))
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        log::debug!("Setting {} raw files", raw_files_as_json_strings.len());
        self.observable.set_state(ChordFilesState {
            raw_files_as_json_strings,
            placeholder_chords,
            loaded_packages,
        })?;

        Ok(())
    }
}

