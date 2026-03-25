use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use llrt_core::{Module, Promise};
use llrt_core::libs::utils::result::ResultExt;
use crate::app::chord_registry::chord_file::AppChordsFileConfig;
use crate::app::chord_registry::chord_package::ChordPackage;
use crate::app::chord_runner::runtime::{ChordRuntime, MatchingChordInfo, MatchingDescriptionInfo, GLOBAL_CHORD_RUNTIME_ID};
use crate::app::placeholder_chord_store::{PlaceholderChordStoreEntry, PlaceholderChordStoreKey};
use crate::app::{AppHandleExt, SafeAppHandle};
use crate::input::Key;
use crate::observables::{ChordFilesObservable, ChordFilesState, Observable, PlaceholderChordInfo};
use crate::quickjs::{reset_js, with_js};

pub mod chord_file;
pub mod chord_package;

pub struct ChordRegistry {
    pub global_chords_to_runtime_key: Mutex<HashMap<Vec<Key>, String>>,
    pub runtimes: Mutex<HashMap<String, Arc<ChordRuntime>>>,

    handle: SafeAppHandle,
    observable: Arc<ChordFilesObservable>,
}


impl ChordRegistry {
    fn parse_packages(
        chord_packages: Vec<ChordPackage>,
        placeholder_entries: &HashMap<PlaceholderChordStoreKey, PlaceholderChordStoreEntry>,
    ) -> anyhow::Result<(
        HashMap<Vec<Key>, String>,
        HashMap<String, ChordRuntime>,
        HashMap<String, serde_json::Value>,
        Vec<PlaceholderChordInfo>,
    )> {
        let mut global_chords_to_runtime_key = HashMap::new();
        let mut app_runtime_map = HashMap::new();
        let mut app_config_map = HashMap::new();
        let mut raw_files_json_map = HashMap::new();
        let mut placeholder_chords = Vec::new();

        for chord_folder in chord_packages {
            if let Some(root_dir) = chord_folder.root_dir {
                log::debug!("Loading folder: {:?}", root_dir);
            } else {
                log::debug!("Loading bundled chords");
            }

            for (chord_file_path, file) in chord_folder.chords_files {
                log::debug!("Loading {:?}", chord_file_path);

                raw_files_json_map.insert(chord_file_path.clone(), file.raw_file_json.clone());

                let Some(runtime_id) = runtime_id_from_chords_path(Path::new(&chord_file_path))
                else {
                    log::warn!("Invalid chords path: {:?}", chord_file_path);
                    continue;
                };

                let placeholder_bindings =
                    placeholder_bindings_for_file(placeholder_entries, &chord_file_path);
                let (scope, scope_kind) = scope_info_from_runtime_id(&runtime_id);

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
                    if is_global_chord_sequence(sequence) {
                        global_chords_to_runtime_key.insert(sequence.clone(), runtime_id.clone());
                    }
                }

                let config = file.config.clone();
                let app_chord_runtime =
                    ChordRuntime::from_file_shallow(chord_file_path, file, &placeholder_bindings)?;
                app_runtime_map.insert(runtime_id.clone(), app_chord_runtime);
                app_config_map.insert(runtime_id, config);
            }

            let application_ids = app_runtime_map.keys().cloned().collect::<Vec<_>>();
            let mut resolved = HashSet::new();
            let mut resolving = HashSet::new();

            for application_id in application_ids {
                resolve_runtime_extends(
                    &application_id,
                    &mut app_runtime_map,
                    &app_config_map,
                    &mut resolved,
                    &mut resolving,
                )?;
            }
        }

        log::debug!(
            "Loaded global chords: {:?}",
            global_chords_to_runtime_key.keys()
        );

        Ok((
            global_chords_to_runtime_key,
            app_runtime_map,
            raw_files_json_map,
            placeholder_chords,
        ))
    }

    pub async fn load_packages(&self, chord_packages: Vec<ChordPackage>) -> anyhow::Result<()> {
        let handle = self.handle.try_handle()?;
        for chord_package in &chord_packages {
            let js_files = chord_package.js_files.clone();
            let root_dir = chord_package.root_dir.clone();

            with_js(handle.clone(), move |ctx| {
                Box::pin(async move {
                    let load_module =
                        |filepath: &String, js: String| -> rquickjs::Result<Promise> {
                            let module_disk_path = module_disk_path(root_dir.as_deref(), &filepath);
                            let module = Module::declare(ctx.clone(), filepath.clone(), js)?;
                            let meta = module.meta()?;
                            meta.set("url", module_disk_path)?;
                            let (_evaluated, promise) = module.eval()?;
                            Ok(promise)
                        };
                    for (filepath, js) in js_files {
                        match load_module(&filepath, js) {
                            Ok(promise) => {
                                if let Err(e) = promise.into_future::<()>().await {
                                    log::error!("failed to await module {}: {:?}", filepath, e);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to load module {}: {:?}", filepath, e);
                            }
                        };
                    }

                    Ok(())
                })
            })
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        let placeholder_entries = handle.app_placeholder_chord_store().entries();
        let (global_chords_to_runtime_key, app_runtime_map, raw_files_map, mut placeholder_chords) =
            ChordRegistry::parse_packages(chord_packages, &placeholder_entries)?;
        placeholder_chords.sort_by(|left, right| {
            left.scope_kind
                .cmp(&right.scope_kind)
                .then(left.scope.cmp(&right.scope))
                .then(left.name.cmp(&right.name))
                .then(left.placeholder.cmp(&right.placeholder))
        });

        // Set state before setting observable
        {
            let mut map = self.global_chords_to_runtime_key.lock().expect("poisoned");
            *map = global_chords_to_runtime_key;
        }

        {
            let mut map = self.runtimes.lock().expect("poisoned");
            *map = app_runtime_map
                .into_iter()
                .map(|(key, value)| (key, Arc::new(value)))
                .collect();
        }

        let raw_files_as_json_strings = raw_files_map
            .iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::to_string(v)?)))
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        log::debug!("Setting {} raw files", raw_files_as_json_strings.len());
        self.observable.set_state(ChordFilesState {
            raw_files_as_json_strings,
            placeholder_chords,
        })?;

        // We should only load `macos.toml` modules AFTER the js files have been loaded
        self.load_chord_config_modules().await;

        Ok(())
    }

    pub fn new_empty(handle: SafeAppHandle, observable: Arc<ChordFilesObservable>) -> Self {
        ChordRegistry {
            handle,
            global_chords_to_runtime_key: Mutex::new(HashMap::new()),
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
            let Some(runtime_key) = global_chords_to_runtime_key.get(sequence) else {
                log::warn!("Invalid global chord sequence: {:?}", sequence);
                return None;
            };

            let runtimes = self.runtimes.lock().expect("poisoned");
            runtimes.get(runtime_key).map(|r| r.clone())
        } else {
            let runtimes = self.runtimes.lock().expect("poisoned");
            application_id.and_then(|app_id| runtimes.get(&app_id).map(|r| r.clone()))
        }
    }

    pub fn list_matching_chords(
        &self,
        key_buffer: &[Key],
        application_id: Option<&str>,
    ) -> Vec<MatchingChordInfo> {
        let (_, chord_prefix) = split_repeat_prefix(key_buffer);
        let mut matches = Vec::new();
        let chords = {
            let lock = self.global_chords_to_runtime_key.lock().expect("poisoned");
            lock.clone()
        };
        let runtimes = self.runtimes.lock().expect("poisoned");

        if chord_prefix.is_empty() {
            if let Some(application_id) = application_id {
                if let Some(runtime) = runtimes.get(application_id) {
                    push_runtime_matches(
                        &mut matches,
                        application_id,
                        "app",
                        runtime.clone(),
                        chord_prefix,
                    );
                }
            }

            for (sequence, runtime_id) in &chords {
                let Some(runtime) = runtimes.get(runtime_id) else {
                    continue;
                };
                let Some(chord) = runtime.chords.get(sequence) else {
                    continue;
                };

                matches.push(MatchingChordInfo {
                    scope: "Global".to_string(),
                    scope_kind: "global",
                    sequence: sequence.clone(),
                    chord: chord.clone(),
                });
            }
        } else if is_global_chord_sequence(chord_prefix) {
            for (sequence, runtime_id) in chords {
                if !sequence.starts_with(chord_prefix) {
                    continue;
                }

                let Some(runtime) = runtimes.get(&runtime_id) else {
                    continue;
                };
                let Some(chord) = runtime.chords.get(&sequence) else {
                    continue;
                };

                matches.push(MatchingChordInfo {
                    scope: "Global".to_string(),
                    scope_kind: "global",
                    sequence: sequence.clone(),
                    chord: chord.clone(),
                });
            }
        } else if let Some(application_id) = application_id {
            if let Some(runtime) = runtimes.get(application_id) {
                push_runtime_matches(
                    &mut matches,
                    application_id,
                    "app",
                    runtime.clone(),
                    chord_prefix,
                );
            }
        }

        matches.sort_by(|left, right| {
            let left_scope_rank = if left.scope_kind == "app" { 0 } else { 1 };
            let right_scope_rank = if right.scope_kind == "app" { 0 } else { 1 };

            left_scope_rank
                .cmp(&right_scope_rank)
                .then(left.sequence.len().cmp(&right.sequence.len()))
                .then(left.scope.cmp(&right.scope))
                .then(left.chord.name.cmp(&right.chord.name))
        });

        matches
    }

    pub fn list_matching_descriptions(
        &self,
        key_buffer: &[Key],
        application_id: Option<&str>,
    ) -> Vec<MatchingDescriptionInfo> {
        let (_, chord_prefix) = split_repeat_prefix(key_buffer);
        let mut matches = Vec::new();

        if chord_prefix.is_empty() {
            if let Some(application_id) = application_id {
                let runtime = {
                    let runtimes = self.runtimes.lock().expect("poisoned");
                    runtimes.get(application_id).map(|r| r.clone())
                };
                if let Some(runtime) = runtime {
                    push_runtime_description_matches(
                        &mut matches,
                        application_id,
                        "app",
                        runtime.clone(),
                        chord_prefix,
                    );
                }
            }

            let global_runtime_ids = {
                let global_chords_to_runtime_key =
                    self.global_chords_to_runtime_key.lock().expect("poisoned");
                global_chords_to_runtime_key
                    .values()
                    .cloned()
                    .collect::<HashSet<_>>()
            };
            for runtime_id in global_runtime_ids {
                let runtime = {
                    let runtimes = self.runtimes.lock().expect("poisoned");
                    runtimes.get(&runtime_id).map(|r| r.clone())
                };
                let Some(runtime) = runtime else {
                    continue;
                };

                push_runtime_description_matches(
                    &mut matches,
                    "Global",
                    "global",
                    runtime.clone(),
                    chord_prefix,
                );
            }
        } else if is_global_chord_sequence(chord_prefix) {
            let global_runtime_ids = {
                let global_chords_to_runtime_key =
                    self.global_chords_to_runtime_key.lock().expect("poisoned");
                global_chords_to_runtime_key
                    .values()
                    .cloned()
                    .collect::<HashSet<_>>()
            };
            for runtime_id in global_runtime_ids {
                let runtime = {
                    let runtimes = self.runtimes.lock().expect("poisoned");
                    runtimes.get(&runtime_id).map(|r| r.clone())
                };
                let Some(runtime) = runtime else {
                    continue;
                };

                push_runtime_description_matches(
                    &mut matches,
                    "Global",
                    "global",
                    runtime.clone(),
                    chord_prefix,
                );
            }
        } else if let Some(application_id) = application_id {
            let runtime = {
                let runtimes = self.runtimes.lock().expect("poisoned");
                runtimes.get(application_id).map(|r| r.clone())
            };
            if let Some(runtime) = runtime {
                push_runtime_description_matches(
                    &mut matches,
                    application_id,
                    "app",
                    runtime.clone(),
                    chord_prefix,
                );
            }
        }

        matches.sort_by(|left, right| {
            let left_scope_rank = if left.scope_kind == "app" { 0 } else { 1 };
            let right_scope_rank = if right.scope_kind == "app" { 0 } else { 1 };

            left_scope_rank
                .cmp(&right_scope_rank)
                .then(left.sequence.len().cmp(&right.sequence.len()))
                .then(left.scope.cmp(&right.scope))
                .then(left.description.cmp(&right.description))
        });

        matches
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

    async fn load_chord_config_modules(&self) {
        let runtimes = {
            let runtimes = self.runtimes.lock().expect("poisoned");
            runtimes.clone()
        };
        for (bundle_id, runtime) in runtimes.iter() {
            let handle = self.handle.clone();

            let Some(js) = runtime.config.as_ref().and_then(|c| c.js.as_ref()) else {
                continue;
            };

            let Some(content) = js.module.clone() else {
                continue;
            };

            let path = runtime.path.clone();
            let raw_chords = runtime.raw_chords.lock().unwrap().clone();
            let bundle_id = bundle_id.clone();

            tauri::async_runtime::spawn(async move {
                let path = path.clone();
                let path2 = path.clone();
                let result = with_js(handle.handle().clone(), move |ctx| {
                    Box::pin(async move {
                        let load_module = async || {
                            let module = Module::declare(ctx.clone(), path.clone(), content)?;

                            let chords =
                                rquickjs_serde::to_value(ctx.clone(), raw_chords).or_throw(&ctx)?;
                            let chords_obj = chords.into_object().or_throw(&ctx)?;

                            let meta = module.meta()?;
                            meta.set("chords", chords_obj)?;
                            meta.set("bundleId", bundle_id.clone())?;

                            let (_evaluated, promise) = module.eval()?;

                            promise
                                .into_future::<()>()
                                .await
                                .or_throw_msg(&ctx, "failed to await module")?;

                            Ok::<(), rquickjs::Error>(())
                        };

                        if let Err(e) = load_module().await {
                            log::error!("Failed to load module {}: {}", path, e);
                        }

                        Ok(())
                    })
                })
                    .await;

                if let Err(err) = result {
                    log::error!("load_module failed for {}: {}", path2, err);
                }
            });
        }
    }
}

fn module_disk_path(root_dir: Option<&Path>, module_path: &str) -> String {
    root_dir
        .map(|root_dir| root_dir.join(module_path))
        .unwrap_or_else(|| PathBuf::from(module_path))
        .display()
        .to_string()
}


fn runtime_id_from_chords_path(file_path: &Path) -> Option<String> {
    if file_path.file_name()? != "macos.toml" {
        return None;
    }

    let application_path = file_path.parent()?.strip_prefix("chords").ok()?;
    if application_path.as_os_str().is_empty() {
        return Some(GLOBAL_CHORD_RUNTIME_ID.to_string());
    }

    Some(
        application_path
            .iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("."),
    )
}

fn is_global_chord_sequence(sequence: &[Key]) -> bool {
    sequence
        .first()
        .is_some_and(|key| !key.is_digit() && !key.is_letter())
}

fn split_repeat_prefix(sequence: &[Key]) -> (&[Key], &[Key]) {
    let split_idx = sequence
        .iter()
        .position(|key| !key.is_digit())
        .unwrap_or(sequence.len());

    sequence.split_at(split_idx)
}

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

fn resolve_runtime_extends(
    application_id: &str,
    app_runtime_map: &mut HashMap<String, ChordRuntime>,
    app_config_map: &HashMap<String, Option<AppChordsFileConfig>>,
    resolved: &mut HashSet<String>,
    resolving: &mut HashSet<String>,
) -> anyhow::Result<()> {
    if resolved.contains(application_id) {
        return Ok(());
    }

    if !resolving.insert(application_id.to_string()) {
        log::warn!("Circular extends detected for application ID: {application_id}");
        return Ok(());
    }

    let extends = app_config_map
        .get(application_id)
        .and_then(|config| config.as_ref())
        .and_then(|config| config.extends.clone());

    if let Some(base_application_id) = extends {
        if app_runtime_map.contains_key(&base_application_id) {
            resolve_runtime_extends(
                &base_application_id,
                app_runtime_map,
                app_config_map,
                resolved,
                resolving,
            )?;

            let Some(mut app_runtime) = app_runtime_map.remove(application_id) else {
                resolving.remove(application_id);
                return Ok(());
            };

            if let Some(base_runtime) = app_runtime_map.get(&base_application_id) {
                app_runtime.extend_runtime(base_runtime)?;
            }

            app_runtime_map.insert(application_id.to_string(), app_runtime);
        } else {
            log::warn!(
                "Invalid extends for application ID {application_id}: {base_application_id}"
            );
        }
    }

    resolving.remove(application_id);
    resolved.insert(application_id.to_string());

    Ok(())
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

fn scope_info_from_runtime_id(runtime_id: &str) -> (String, String) {
    if runtime_id == GLOBAL_CHORD_RUNTIME_ID {
        ("Global".to_string(), "global".to_string())
    } else {
        (runtime_id.to_string(), "app".to_string())
    }
}
