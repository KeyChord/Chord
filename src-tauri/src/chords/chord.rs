use crate::chords::shortcut::{Shortcut, press_shortcut, release_shortcut};
use crate::chords::{AppChordMapValue, AppChordsFile, AppChordsFileConfig, ChordPackage};
use crate::feature::SafeAppHandle;
use crate::feature::app_handle_ext::AppHandleExt;
use crate::input::Key;
use crate::js::{format_js_error, reset_js, with_js};
use crate::observables::{ChordFilesObservable, ChordFilesState, Observable};
use anyhow::Result;
use llrt_core::libs::utils::result::ResultExt;
use rquickjs::function::Args;
use rquickjs::{Array, Ctx, Function, Module, Object, Promise, Value};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum ChordJsArgs {
    #[typeshare(typescript(type = "any"))]
    Values(Vec<toml::Value>),
    Eval(String),
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChordJsInvocation {
    pub export_name: Option<String>,
    pub args: ChordJsArgs,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub name: String,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
    pub js: Option<ChordJsInvocation>,
}

pub struct ChordRegistry {
    pub global_chords_to_runtime_key: Mutex<HashMap<Vec<Key>, String>>,
    pub runtimes: Mutex<HashMap<String, Arc<ChordRuntime>>>,

    handle: SafeAppHandle,
    observable: Arc<ChordFilesObservable>,
}

#[derive(Debug, Clone)]
pub struct MatchingChordInfo {
    pub scope: String,
    pub scope_kind: &'static str,
    pub sequence: Vec<Key>,
    pub chord: Chord,
}

#[derive(Debug, Clone)]
pub struct MatchingDescriptionInfo {
    pub scope: String,
    pub scope_kind: &'static str,
    pub sequence: Vec<Key>,
    pub description: String,
}

// Each chord runtime is associated with a JS module which lives in-memory
// (similar to require.cache)
pub struct ChordRuntime {
    // Used as a unique module key
    pub path: String,

    pub chords: HashMap<Vec<Key>, Chord>,
    pub descriptions: HashMap<Vec<Key>, String>,
    // Needs to be an Arc so the JS runtime can access its latest value
    pub raw_chords: Arc<Mutex<HashMap<String, AppChordMapValue>>>,
    pub config: Option<AppChordsFileConfig>,
}

#[derive(Debug, Clone)]
pub struct ChordPayload {
    pub chord: Chord,
    pub num_times: usize,
}

pub(crate) const GLOBAL_CHORD_RUNTIME_ID: &str = "__global__";

impl ChordRuntime {
    pub fn from_chords(path: String, chords: HashMap<Vec<Key>, Chord>) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(HashMap::new()));
        Ok(Self {
            path,
            chords,
            descriptions: HashMap::new(),
            raw_chords,
            config: None,
        })
    }

    // Doesn't resolve _config.extends
    pub fn from_file_shallow(path: String, chord_file: AppChordsFile) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(chord_file.chords.clone()));
        let config = chord_file.config.clone();

        // We intentionally keep global chords because they execute in this runtime
        let chords = chord_file.get_chords_shallow();
        let descriptions = chord_file.get_descriptions_shallow();

        Ok(Self {
            path,
            raw_chords,
            config,
            chords,
            descriptions,
        })
    }

    pub fn extend_runtime(&mut self, base: &Self) -> Result<()> {
        for (sequence, chord) in &base.chords {
            self.chords
                .entry(sequence.clone())
                .or_insert_with(|| chord.clone());
        }

        for (sequence, description) in &base.descriptions {
            self.descriptions
                .entry(sequence.clone())
                .or_insert_with(|| description.clone());
        }

        let mut raw_chords = self.raw_chords.lock().expect("poisoned lock");
        let base_raw_chords = base.raw_chords.lock().expect("poisoned lock");
        for (sequence, chord) in base_raw_chords.iter() {
            raw_chords
                .entry(sequence.clone())
                .or_insert_with(|| chord.clone());
        }

        Ok(())
    }

    pub fn get_chord(&self, sequence: &[Key]) -> Option<ChordPayload> {
        let split_idx = sequence
            .iter()
            .position(|k| !k.is_digit())
            .unwrap_or(sequence.len());
        let (digit_keys, chord_keys) = sequence.split_at(split_idx);
        let num_times = if digit_keys.is_empty() {
            1
        } else {
            let digits: String = digit_keys.iter().filter_map(|k| k.to_char(false)).collect();
            let num_times = digits.parse::<usize>().unwrap_or(1);
            num_times
        };
        self.chords.get(chord_keys).map(|chord| ChordPayload {
            chord: chord.clone(),
            num_times,
        })
    }
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
) -> Result<()> {
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

impl ChordRegistry {
    fn parse_packages(
        chord_packages: Vec<ChordPackage>,
    ) -> Result<(
        HashMap<Vec<Key>, String>,
        HashMap<String, ChordRuntime>,
        HashMap<String, serde_json::Value>,
    )> {
        let mut global_chords_to_runtime_key = HashMap::new();
        let mut app_runtime_map = HashMap::new();
        let mut app_config_map = HashMap::new();
        let mut raw_files_json_map = HashMap::new();

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

                let chords = file.get_chords_shallow();
                for sequence in chords.keys() {
                    if is_global_chord_sequence(sequence) {
                        global_chords_to_runtime_key.insert(sequence.clone(), runtime_id.clone());
                    }
                }

                let config = file.config.clone();
                let app_chord_runtime = ChordRuntime::from_file_shallow(chord_file_path, file)?;
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
        ))
    }

    pub async fn load_packages(&self, chord_packages: Vec<ChordPackage>) -> Result<()> {
        let Some(handle) = self.handle.try_handle() else {
            anyhow::bail!("app not ready")
        };

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

        let (global_chords_to_runtime_key, app_runtime_map, raw_files_map) =
            ChordRegistry::parse_packages(chord_packages)?;

        // Set state before setting observable
        {
            let mut map = self.global_chords_to_runtime_key.lock().expect("poisoned");
            map.extend(global_chords_to_runtime_key);
        }

        {
            let mut map = self.runtimes.lock().expect("poisoned");
            map.extend(
                app_runtime_map
                    .into_iter()
                    .map(|(key, value)| (key, Arc::new(value))),
            );
        }

        let state = self.observable.get_state()?;
        let mut raw_files_as_json_strings = state.raw_files_as_json_strings.clone();
        let new_entries = raw_files_map
            .iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::to_string(v)?)))
            .collect::<Result<Vec<_>>>()?;
        raw_files_as_json_strings.extend(new_entries);
        log::debug!("Setting {} raw files", raw_files_as_json_strings.len());
        self.observable.set_state(ChordFilesState {
            raw_files_as_json_strings,
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
    pub async fn reload(&self) -> Result<()> {
        let Some(handle) = self.handle.try_handle() else {
            anyhow::bail!("app not loaded yet")
        };

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
                let path_ = path.clone();
                let result = with_js(handle.handle().clone(), move |ctx| {
                    Box::pin(async move {
                        let load_module = || -> rquickjs::Result<rquickjs::Promise> {
                            let module = Module::declare(ctx.clone(), path.clone(), content)?;
                            let chords =
                                rquickjs_serde::to_value(ctx.clone(), raw_chords).or_throw(&ctx)?;
                            let chords_obj = chords.into_object().or_throw(&ctx);
                            let meta = module.meta()?;
                            meta.set("chords", chords_obj)?;
                            meta.set("bundleId", bundle_id)?;
                            let (_evaluated, promise) = module.eval()?;
                            Ok(promise)
                        };

                        match load_module() {
                            Ok(promise) => {
                                if let Err(e) = promise.into_future::<()>().await {
                                    log::error!(
                                        "failed to await module {}: {}",
                                        path,
                                        format_js_error(ctx.clone(), e)
                                    )
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to load module {}: {}",
                                    path,
                                    format_js_error(ctx.clone(), e)
                                );
                            }
                        }

                        Ok(())
                    })
                })
                .await;

                if let Err(err) = result {
                    log::error!("load_module failed for {}: {}", path_, err);
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

fn press_shortcut_on_main_thread(
    handle: AppHandle,
    shortcut: Shortcut,
    num_times: usize,
) -> Result<()> {
    handle.run_on_main_thread(move || {
        if let Err(e) = press_shortcut(shortcut.clone(), num_times) {
            log::error!("failed to press shortcut: {e}");
        }
    })?;

    Ok(())
}

fn release_shortcut_on_main_thread(handle: AppHandle, shortcut: Shortcut) -> Result<()> {
    handle.run_on_main_thread(move || {
        if let Err(e) = release_shortcut(shortcut.clone()) {
            log::error!("failed to release shortcut: {e}");
        }
    })?;

    Ok(())
}

fn run_shell_command_in_background(shell: String) {
    std::thread::spawn(move || run_shell_command(shell));
}

fn run_shell_command(shell: String) {
    let mut command = Command::new("sh");
    command.arg("-c").arg(&shell);
    log::debug!("Running shell command: {:?}", command);

    match command.output() {
        Ok(output) => log_shell_output(&shell, output),
        Err(e) => {
            log::error!("failed to run shell command `{shell}`: {e}");
        }
    }
}

fn log_shell_output(shell: &str, output: std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let exit_code = output.status.code();

    if output.status.success() {
        log::debug!(
            "shell command succeeded with exit code {:?}: {}",
            exit_code,
            shell
        );
    } else {
        log::error!(
            "shell command failed with exit code {:?}: {}",
            exit_code,
            shell
        );
    }

    if !stdout.is_empty() {
        log::debug!("shell stdout: {stdout}");
    }

    if !stderr.is_empty() {
        log::debug!("shell stderr: {stderr}");
    }
}

fn invoke_js_chord_in_background(
    handle: AppHandle,
    module_path: String,
    invocation: ChordJsInvocation,
    num_times: usize,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = with_js(handle.clone(), move |ctx| {
            Box::pin(call_js_export(ctx, module_path, invocation, num_times))
        })
        .await
        {
            log::error!("press_chord failed: {}", e);
        }
    });
}

async fn call_js_export<'js>(
    ctx: Ctx<'js>,
    module_path: String,
    invocation: ChordJsInvocation,
    num_times: usize,
) -> anyhow::Result<()> {
    for _ in 0..num_times {
        let Some(namespace) = import_js_namespace(ctx.clone(), &module_path).await else {
            return Ok(());
        };

        let Some(function) =
            get_export_function(ctx.clone(), &namespace, invocation.export_name.as_deref()).await
        else {
            return Ok(());
        };

        let Some(js_args) = convert_js_args(&ctx, invocation.args.clone()) else {
            return Ok(());
        };

        let export_name = invocation.export_name.as_deref().unwrap_or("default");
        log::debug!(
            "Calling JS export `{}` with arguments: {:?}",
            export_name,
            js_args
        );

        let result = match call_function_with_values(ctx.clone(), function, js_args) {
            Ok(value) => value,
            Err(e) => {
                log::error!(
                    "Failed to call JS export `{}`: {}",
                    export_name,
                    format_js_error(ctx.clone(), e)
                );
                return Ok(());
            }
        };

        log::debug!("Return value: {:?}", result);

        match await_promise_if_needed(ctx.clone(), result).await {
            Ok(awaited) => {
                log::debug!("Promise awaited: {:?}", awaited);
            }
            Err(e) => {
                log::error!(
                    "JS export `{}` promise rejected: {}",
                    export_name,
                    format_js_error(ctx.clone(), e)
                );
            }
        }
    }

    Ok(())
}

async fn import_js_namespace<'js>(ctx: Ctx<'js>, module_path: &str) -> Option<Object<'js>> {
    let import_promise = match Module::import(&ctx, module_path.to_string()) {
        Ok(import_promise) => import_promise,
        Err(e) => {
            log::error!(
                "Failed to start importing JS module: {}",
                format_js_error(ctx.clone(), e)
            );
            return None;
        }
    };

    match import_promise.into_future::<Object>().await {
        Ok(namespace) => Some(namespace),
        Err(e) => {
            log::error!(
                "Failed to import JS module {}: {}",
                module_path,
                format_js_error(ctx.clone(), e)
            );
            None
        }
    }
}

async fn get_export_function<'js>(
    ctx: Ctx<'js>,
    namespace: &Object<'js>,
    export_name: Option<&str>,
) -> Option<Function<'js>> {
    let export_name = export_name.unwrap_or("default");
    let export: Value<'js> = match namespace.get(export_name) {
        Ok(export) => export,
        Err(e) => {
            log::error!(
                "Failed to get JS export `{}`: {}",
                export_name,
                format_js_error(ctx.clone(), e)
            );
            return None;
        }
    };

    log::debug!("JS export `{}`: {:?}", export_name, export);
    let resolved: Value<'js> = if let Some(promise) = export.as_promise().cloned() {
        match promise.into_future::<Value<'js>>().await {
            Ok(value) => value,
            Err(e) => {
                log::error!(
                    "Failed to resolve JS export `{}` promise: {}",
                    export_name,
                    format_js_error(ctx.clone(), e)
                );
                return None;
            }
        }
    } else {
        export
    };

    let Some(function) = resolved.as_function().cloned() else {
        log::error!(
            "JS export `{}` did not resolve to a function: {:?}",
            export_name,
            resolved
        );
        return None;
    };

    Some(function)
}

fn convert_js_args<'js>(ctx: &Ctx<'js>, args: ChordJsArgs) -> Option<Vec<Value<'js>>> {
    match args {
        ChordJsArgs::Values(values) => toml_values_to_js_args(ctx, values),
        ChordJsArgs::Eval(source) => evaluate_js_args(ctx, &source),
    }
}

fn toml_values_to_js_args<'js>(
    ctx: &Ctx<'js>,
    values: Vec<toml::Value>,
) -> Option<Vec<Value<'js>>> {
    let mut js_args = Vec::with_capacity(values.len());

    for value in values {
        match rquickjs_serde::to_value(ctx.clone(), value) {
            Ok(value) => js_args.push(value),
            Err(e) => {
                log::error!("Failed to convert TOML arguments: {}", e);
                return None;
            }
        }
    }

    Some(js_args)
}

fn evaluate_js_args<'js>(ctx: &Ctx<'js>, source: &str) -> Option<Vec<Value<'js>>> {
    let evaluated: Value<'js> = match ctx.eval(source) {
        Ok(value) => value,
        Err(e) => {
            log::error!(
                "Failed to evaluate JS args `{}`: {}",
                source,
                format_js_error(ctx.clone(), e)
            );
            None
        }
    }?;

    let Some(array) = value_to_array(ctx, evaluated, source) else {
        return None;
    };

    array_to_values(ctx, array, source)
}

fn value_to_array<'js>(_ctx: &Ctx<'js>, value: Value<'js>, source: &str) -> Option<Array<'js>> {
    let Some(array) = value.as_array().cloned() else {
        log::error!("JS args `{}` must evaluate to an array", source);
        return None;
    };

    Some(array)
}

fn array_to_values<'js>(
    ctx: &Ctx<'js>,
    array: Array<'js>,
    source: &str,
) -> Option<Vec<Value<'js>>> {
    let mut values = Vec::with_capacity(array.len());

    for index in 0..array.len() {
        match array.get(index) {
            Ok(value) => values.push(value),
            Err(e) => {
                log::error!(
                    "Failed to read JS args `{}` at index {}: {}",
                    source,
                    index,
                    format_js_error(ctx.clone(), e)
                );
                return None;
            }
        }
    }

    Some(values)
}

fn call_function_with_values<'js>(
    ctx: Ctx<'js>,
    function: Function<'js>,
    values: Vec<Value<'js>>,
) -> rquickjs::Result<Value<'js>> {
    let mut args_builder = Args::new(ctx, values.len());

    for value in values {
        args_builder.push_arg(value)?;
    }

    function.call_arg(args_builder)
}

async fn await_promise_if_needed<'js>(ctx: Ctx<'js>, result: Value<'js>) -> rquickjs::Result<()> {
    if !result.is_promise() {
        return Ok(());
    }

    let promise = match Promise::from_value(result) {
        Ok(promise) => promise,
        Err(e) => {
            log::error!(
                "Function returned something marked as promise, but it could not be converted: {}",
                format_js_error(ctx.clone(), e)
            );
            return Ok(());
        }
    };

    let result = promise.into_future::<Value>().await.map(|_| ());
    log::debug!("Promise result: {:?}", result);
    result
}

pub fn press_chord(
    handle: AppHandle,
    runtime: Arc<ChordRuntime>,
    chord_payload: &ChordPayload,
) -> Result<()> {
    log::debug!("Pressing chord: {:?}", chord_payload);

    if let Some(shortcut) = chord_payload.chord.shortcut.clone() {
        return press_shortcut_on_main_thread(handle, shortcut, chord_payload.num_times);
    }

    if let Some(shell) = chord_payload.chord.shell.clone() {
        run_shell_command_in_background(shell);
        return Ok(());
    }

    if let Some(js) = chord_payload.chord.js.clone() {
        invoke_js_chord_in_background(handle, runtime.path.clone(), js, chord_payload.num_times);
    }

    Ok(())
}

pub fn release_chord(handle: AppHandle, chord: &Chord) -> Result<()> {
    if let Some(shortcut) = chord.shortcut.clone() {
        release_shortcut_on_main_thread(handle, shortcut)?;
    }

    Ok(())
}
