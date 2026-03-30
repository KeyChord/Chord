use std::collections::HashMap;
use std::path::PathBuf;
use fast_radix_trie::StringRadixMap;
use llrt_core::Module;
use crate::app::chord_package_manager::{ChordJsPackage, ChordPackage, ChordReference};
use crate::app::chord_package_manager::registry::ChordPackageRegistry;
use crate::input::Key;
use crate::models::{ChordInput, ChordsFile, RawChordPackage};
use crate::quickjs::{format_js_error, with_js};
use anyhow::{Context, Result};
use gix::bstr::ByteVec;
use parking_lot::RwLock;
use tauri::AppHandle;
use crate::app::state::StateSingleton;
use crate::observables::{ChordPackageManagerObservable, ChordPackageManagerState, Observable};

pub struct ChordPackageManager {
    packages: RwLock<HashMap<String, ChordPackage>>,
    pub registry: ChordPackageRegistry,

    observable: ChordPackageManagerObservable,
    handle: AppHandle,
}

impl StateSingleton for ChordPackageManager {
    fn new(handle: AppHandle) -> Self {
        Self {
            packages: RwLock::new(HashMap::new()),
            registry: ChordPackageRegistry::new(handle.clone()),
            observable: ChordPackageManagerObservable::placeholder(),
            handle
        }
    }
}

impl ChordPackageManager {
    pub fn init(&self, observable: ChordPackageManagerObservable) -> Result<()> {
        self.observable.init(observable);
        Ok(())
    }

    pub async fn reload_all(&self) -> Result<()> {
        let raw_chord_packages = self.registry.import_all_packages()?;
        self.packages.write().clear();

        let mut chord_packages = Vec::new();
        for raw_chord_package in raw_chord_packages {
            chord_packages.push(self.load_package(raw_chord_package).await?);
        }
        self.observable.set_state(ChordPackageManagerState {
            packages: chord_packages
        })?;

        Ok(())
    }

    pub fn get_package_by_name(&self, package_name: &str) -> Option<ChordPackage> {
        self.packages.read().get(package_name).cloned()
    }

    pub async fn load_package(&self, raw_chord_package: RawChordPackage) -> Result<ChordPackage> {
        let name = self.get_package_name(&raw_chord_package)?;
        log::debug!("loading package {}", name);

        let mut app_chords_files = HashMap::new();
        let mut global_chords = Vec::new();

        for (path, contents) in raw_chord_package.chords_files_contents {
            let Ok(chords_file) = contents.parse::<ChordsFile>().inspect_err(|e| {
                log::error!("error when loading package {}; failed to parse chords file {}:\n{}", name, e, contents);
            }) else {
                continue;
            };

            for chord in &chords_file.chords {
                let first_char = chord.raw_trigger.chars().next();
                let is_non_alphanumeric = first_char.map(|c| !c.is_alphanumeric()).unwrap_or(false);

                if is_non_alphanumeric {
                    global_chords.push(ChordReference {
                        package_name: name.clone(),
                        chords_file_path: path.clone(),
                        chord: chord.clone()
                    });
                }
            }

            app_chords_files.insert(path, chords_file.clone());
        }

        let js_package = self.load_js_package(
            &name,
            &raw_chord_package.js_files_contents
        ).await?;

        let chord_package = ChordPackage {
            name: name.clone(),
            js_package,
            app_chords_files,
            global_chords
        };

        self.packages.write().insert(name, chord_package.clone());

        Ok(chord_package)
    }

    async fn load_js_package(&self, package_name: &str, files: &HashMap<PathBuf, String>) -> Result<Option<ChordJsPackage>> {
        log::debug!("loading JS package {}", package_name);

        if files.is_empty() {
            log::debug!("JS package {} was empty", package_name);
            return Ok(None);
        }

        let mut exported_files = HashMap::new();

        for (file_relpath, js) in files.iter() {
            exported_files.insert(file_relpath.clone(), js.clone());
            let file_relpath = file_relpath.to_owned();
            let js_string = js.clone();
            let path = PathBuf::from(package_name).join(file_relpath.clone());
            let module_specifier = path.to_str().context("invalid path")?.to_string();
            with_js(self.handle.clone(), move |ctx| {
                Box::pin(async move {
                    log::debug!("declaring module {}", module_specifier);
                    let module = Module::declare(ctx.clone(), module_specifier.clone(), js_string.clone()).map_err(
                        |e| anyhow::format_err!(
                            "error declaring module {:?}\nerror:{}\nfile:\n{}",
                            module_specifier,
                            format_js_error(&ctx, e),
                            format!("{}...", js_string.chars().take(100).collect::<String>())
                        )
                    )?;
                    let meta = module.meta()?;
                    meta.set("url", file_relpath.to_str())?;
                    let (_evaluated, promise) = module.eval()?;
                    promise.into_future::<()>().await?;
                    Ok(())
                })
            })
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        };

        Ok(Some(ChordJsPackage::new(exported_files)))
    }

    fn get_package_name_from_package_json(&self, raw_chord_package: &RawChordPackage) -> anyhow::Result<Option<String>> {
        if let Some(package_json_contents) = &raw_chord_package.package_json_contents {
            let json: serde_json::Value = serde_json::from_str(package_json_contents)?;
            Ok(json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }

    fn get_package_name(&self, raw_chord_package: &RawChordPackage) -> Result<String> {
        if let Some(name) = self.get_package_name_from_package_json(raw_chord_package)? {
            Ok(name)
        } else {
            Ok(raw_chord_package.dirname.clone())
        }
    }

    pub fn resolve_package_for_input(&self, input: &ChordInput) -> Option<ChordPackage> {
        let packages = self.packages.read();

        if let Some(app_id) = &input.application_id {
            let path = format!("chords/{}/macos.toml", app_id.replace(".", "/"));
            let path = PathBuf::from(path);
            for package in packages.values() {
                if let Some(chords_file) = package.app_chords_files.get(&path) {
                    if chords_file.chords.iter().find(|c| c.trigger.matches(&input.keys)).is_some() {
                        return Some(package.clone())
                    }
                }
            }
        }

        let sequence_str = Key::serialize_sequence(&input.keys)?;
        for package in packages.values() {
            if package.global_chords.iter().find(|c| c.chord.raw_trigger == sequence_str).is_some() {
                return Some(package.clone());
            }
        }

        None
    }
}

fn get_package_name(specifier: &str) -> &str {
    if specifier.starts_with('@') {
        // Scoped: @scope/name/...
        let mut parts = specifier.splitn(3, '/');
        match (parts.next(), parts.next()) {
            (Some(scope), Some(name)) => {
                // return "@scope/name"
                let len = scope.len() + 1 + name.len();
                &specifier[..len]
            }
            _ => specifier, // fallback if malformed
        }
    } else {
        // Unscoped: name/...
        specifier.split('/').next().unwrap_or(specifier)
    }
}
