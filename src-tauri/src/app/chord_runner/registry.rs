use crate::app::chord_package::ChordPackage;
use crate::app::chord_runner::runtime::{
    ChordRuntime, GLOBAL_CHORD_RUNTIME_ID, MatchingChordInfo, MatchingDescriptionInfo,
};
use crate::app::placeholder_chord_store::{PlaceholderChordStoreEntry, PlaceholderChordStoreKey};
use crate::app::{AppHandleExt, SafeAppHandle};
use crate::input::Key;
use crate::observables::{
    ChordFilesObservable, ChordFilesState, GitReposObservable, LoadedChordPackageInfo, Observable,
    PlaceholderChordInfo,
};
use crate::quickjs::{format_js_error, reset_js, with_js};
use llrt_core::Module;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct ChordRunnerRegistry {
    pub global_chords_to_runtime_key: Mutex<HashMap<Vec<Key>, String>>,
    pub runtime_index: Mutex<HashMap<String, Arc<ChordRuntime>>>,
    pub runtimes: Mutex<HashMap<String, Vec<Arc<ChordRuntime>>>>,

    handle: SafeAppHandle,
    observable: Arc<ChordFilesObservable>,
}

struct RuntimePathInfo {
    runtime_id: String,
    bundle_id: String,
}

impl ChordRunnerRegistry {
    pub fn new_empty(handle: SafeAppHandle, observable: Arc<ChordFilesObservable>) -> Self {
        ChordRunnerRegistry {
            handle,
            global_chords_to_runtime_key: Mutex::new(HashMap::new()),
            runtime_index: Mutex::new(HashMap::new()),
            runtimes: Mutex::new(HashMap::new()),
            observable,
        }
    }

    // No application = global chord
    pub fn get_chord_runtime(
        &self,
        sequence: &[Key],
        application_id: Option<String>,
    ) -> Option<Arc<ChordRuntime>> {
        if is_global_chord_sequence(sequence) {
            let global_chords_to_runtime_key =
                self.global_chords_to_runtime_key.lock().expect("poisoned");
            if let Some(runtime_key) = global_chords_to_runtime_key.get(sequence) {
                let runtime_index = self.runtime_index.lock().expect("poisoned");
                return runtime_index.get(runtime_key).map(|r| r.clone());
            }

            let runtimes = self.runtimes.lock().expect("poisoned");
            runtimes
                .get(GLOBAL_CHORD_RUNTIME_ID)
                .and_then(|global_runtimes| {
                    global_runtimes
                        .iter()
                        .rev()
                        .find(|runtime| runtime.get_chord(sequence).is_some())
                        .cloned()
                })
        } else {
            let runtimes = self.runtimes.lock().expect("poisoned");
            application_id.and_then(|app_id| {
                runtimes.get(&app_id).and_then(|app_runtimes| {
                    app_runtimes
                        .iter()
                        .rev()
                        .find(|runtime| runtime.get_chord(sequence).is_some())
                        .cloned()
                })
            })
        }
    }

    /// Also re-evaluates JavaScript
    pub async fn reload(&self) -> anyhow::Result<()> {
        let handle = self.handle.try_handle()?;
        let chorder = handle.app_chorder();
        let chord_package_registry = handle.app_chord_package_registry();
        chorder.ensure_inactive()?;

        let chord_packages = chord_package_registry.load_all_chord_packages()?;
        reset_js(handle.clone()).await?;
        self.load_packages(chord_packages).await?;

        Ok(())
    }
}

fn module_disk_path(root_dir: Option<&Path>, module_path: &str) -> String {
    root_dir
        .map(|root_dir| root_dir.join(module_path))
        .unwrap_or_else(|| PathBuf::from(module_path))
        .display()
        .to_string()
}

fn runtime_info_from_chords_path(file_path: &Path) -> Option<RuntimePathInfo> {
    let file_name = file_path.file_name()?.to_str()?;
    if !is_supported_macos_chord_filename(file_name) {
        return None;
    }

    let application_path = file_path.parent()?.strip_prefix("chords").ok()?;
    let bundle_id = if application_path.as_os_str().is_empty() {
        GLOBAL_CHORD_RUNTIME_ID.to_string()
    } else {
        application_path
            .iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(".")
    };
    let runtime_id = if file_name == "macos.toml" {
        bundle_id.clone()
    } else {
        let runtime_name = file_name.strip_suffix(".macos.toml")?;
        format!("{bundle_id}#{runtime_name}")
    };

    Some(RuntimePathInfo {
        runtime_id,
        bundle_id,
    })
}

fn is_global_chord_sequence(sequence: &[Key]) -> bool {
    sequence
        .first()
        .is_some_and(|key| !key.is_digit() && !key.is_letter())
}

#[allow(dead_code)]
fn split_repeat_prefix(sequence: &[Key]) -> (&[Key], &[Key]) {
    let split_idx = sequence
        .iter()
        .position(|key| !key.is_digit())
        .unwrap_or(sequence.len());

    sequence.split_at(split_idx)
}

#[allow(dead_code)]
fn push_runtime_matches(
    matches: &mut Vec<MatchingChordInfo>,
    scope: &str,
    scope_kind: &'static str,
    runtime: Arc<ChordRuntime>,
    chord_prefix: &[Key],
) {
    for (sequence, chord) in &runtime.chords {
        if !sequence.starts_with(chord_prefix) {
            continue;
        }

        matches.push(MatchingChordInfo {
            scope: scope.to_string(),
            scope_kind,
            sequence: sequence.clone(),
            chord: chord.clone(),
        });
    }
}

#[allow(dead_code)]
fn push_runtime_description_matches(
    matches: &mut Vec<MatchingDescriptionInfo>,
    scope: &str,
    scope_kind: &'static str,
    runtime: Arc<ChordRuntime>,
    chord_prefix: &[Key],
) {
    for (sequence, description) in &runtime.descriptions {
        if !sequence.starts_with(chord_prefix) {
            continue;
        }

        matches.push(MatchingDescriptionInfo {
            scope: scope.to_string(),
            scope_kind,
            sequence: sequence.clone(),
            description: description.clone(),
        });
    }
}

fn placeholder_bindings_for_file(
    entries: &HashMap<PlaceholderChordStoreKey, PlaceholderChordStoreEntry>,
    file_path: &str,
) -> HashMap<String, String> {
    entries
        .iter()
        .filter_map(|(key, entry)| {
            (key.file_path == file_path)
                .then_some((key.sequence_template.clone(), entry.sequence.clone()))
        })
        .collect()
}

fn is_supported_macos_chord_filename(file_name: &str) -> bool {
    file_name == "macos.toml" || file_name.ends_with(".macos.toml")
}

fn scope_info_from_bundle_id(bundle_id: &str) -> (String, String) {
    if bundle_id == GLOBAL_CHORD_RUNTIME_ID {
        ("Global".to_string(), "global".to_string())
    } else {
        (bundle_id.to_string(), "app".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_base_runtime_info_from_chords_path() {
        let info = runtime_info_from_chords_path(Path::new("chords/com/apple/finder/macos.toml"))
            .expect("runtime info");

        assert_eq!(info.bundle_id, "com.apple.finder");
        assert_eq!(info.runtime_id, "com.apple.finder");
    }

    #[test]
    fn parses_named_runtime_info_from_chords_path() {
        let info =
            runtime_info_from_chords_path(Path::new("chords/com/apple/finder/work.macos.toml"))
                .expect("runtime info");

        assert_eq!(info.bundle_id, "com.apple.finder");
        assert_eq!(info.runtime_id, "com.apple.finder#work");
    }

    #[test]
    fn parses_global_named_runtime_info_from_chords_path() {
        let info = runtime_info_from_chords_path(Path::new("chords/work.macos.toml"))
            .expect("runtime info");

        assert_eq!(info.bundle_id, GLOBAL_CHORD_RUNTIME_ID);
        assert_eq!(info.runtime_id, "__global__#work");
    }
}
