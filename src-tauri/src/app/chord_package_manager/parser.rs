use std::collections::HashMap;
use std::path::Path;
use crate::app::chord_package::raw::ChordPackage;
use crate::app::chord_runner::runtime::ChordRuntime;
use crate::app::placeholder_chord_store::{PlaceholderChordStoreEntry, PlaceholderChordStoreKey};
use crate::input::Key;
use crate::observables::PlaceholderChordInfo;

struct ChordPackageParser {}

struct ParsedChordPackage {
    global_chords_to_runtime_key: HashMap<Vec<Key>, String>,
    app_runtime_map: HashMap<String, ChordRuntime>,
    runtime_bundle_ids: HashMap<String, String>,
    runtime_order: Vec<String>,
    raw_files_json_map: HashMap<String, serde_json::Value>,
    placeholder_chords: Vec<PlaceholderChordInfo>,
}

impl ChordPackageParser {

    fn parse_packages(
        chord_packages: Vec<ChordPackage>,
        placeholder_entries: &HashMap<PlaceholderChordStoreKey, PlaceholderChordStoreEntry>,
    ) -> anyhow::Result<ParsedChordPackage> {
        let mut global_chords_to_runtime_key = HashMap::new();
        let mut app_runtime_map = HashMap::new();
        let mut runtime_bundle_ids = HashMap::new();
        let mut runtime_order = Vec::new();
        let mut raw_files_json_map = HashMap::new();
        let mut placeholder_chords = Vec::new();

        for chord_folder in chord_packages {
            if let Some(root_dir) = chord_folder.root_dir {
                log::debug!("Loading folder: {:?}", root_dir);
            }

            for (chord_file_path, file) in chord_folder.chords_files {
                log::debug!("Loading {:?}", chord_file_path);

                raw_files_json_map.insert(chord_file_path.clone(), file.raw_file_json.clone());

                let Some(runtime_info) = crate::app::chord_runner::registry::runtime_info_from_chords_path(Path::new(&chord_file_path))
                else {
                    log::warn!("Invalid chords path: {:?}", chord_file_path);
                    continue;
                };
                let runtime_id = runtime_info.runtime_id;
                let bundle_id = runtime_info.bundle_id;

                let placeholder_bindings =
                    crate::app::chord_runner::registry::placeholder_bindings_for_file(placeholder_entries, &chord_file_path);
                let (scope, scope_kind) = crate::app::chord_runner::registry::scope_info_from_bundle_id(&bundle_id);

                placeholder_chords.extend(file.placeholder_chords.iter().filter_map(
                    |placeholder| {
                        let Some(name) = placeholder.name() else {
                            return None;
                        };

                        Some(PlaceholderChordInfo {
                            file_path: chord_file_path.clone(),
                            scope: scope.clone(),
                            scope_kind: scope_kind.clone(),
                            name,
                            placeholder: placeholder.placeholder.clone(),
                            sequence_template: placeholder.sequence_template.clone(),
                            sequence_prefix: placeholder.sequence_prefix.clone(),
                            sequence_suffix: placeholder.sequence_suffix.clone(),
                            assigned_sequence: placeholder_bindings
                                .get(&placeholder.sequence_template)
                                .cloned(),
                        })
                    },
                ));

                let chords = file.get_chords_shallow(&placeholder_bindings);
                for sequence in chords.keys() {
                    if crate::app::chord_runner::registry::is_global_chord_sequence(sequence) {
                        global_chords_to_runtime_key.insert(sequence.clone(), runtime_id.clone());
                    }
                }

                let app_chord_runtime = ChordRuntime::from_file_shallow(
                    chord_file_path,
                    file,
                    &placeholder_bindings,
                )?;
                app_runtime_map.insert(runtime_id.clone(), app_chord_runtime);
                runtime_bundle_ids.insert(runtime_id.clone(), bundle_id);
                runtime_order.push(runtime_id);
            }
        }

        log::debug!(
            "Loaded global chords: {:?}",
            global_chords_to_runtime_key.keys()
        );

        Ok((
            global_chords_to_runtime_key,
            app_runtime_map,
            runtime_bundle_ids,
            runtime_order,
            raw_files_json_map,
            placeholder_chords,
        ))
    }
}