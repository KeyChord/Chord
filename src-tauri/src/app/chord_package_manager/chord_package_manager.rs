use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::app::chord_package_manager::registry::ChordPackageRegistry;
use crate::app::chord_package_manager::{ChordJsPackage, ChordPackage, ChordReference};
use crate::app::state::StateSingleton;
use crate::models::{ChordInput, ParsedChordsFile, RawChordPackage, RawChordsFile};
use crate::observables::{ChordPackageManagerObservable, ChordPackageManagerState, Observable};
use crate::quickjs::{format_js_error, with_js};
use anyhow::{Context, Result};
use llrt_core::Module;
use parking_lot::RwLock;
use tauri::AppHandle;

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
            handle,
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
            let package_name = self
                .get_package_name(&raw_chord_package)
                .unwrap_or(raw_chord_package.dirname.clone());
            match self.load_package(raw_chord_package).await {
                Ok(package) => {
                    chord_packages.push(package);
                }
                Err(e) => {
                    log::error!(
                        "skipping package {} because of loading error: {:?}",
                        package_name,
                        e
                    )
                }
            };
        }
        self.observable.set_state(ChordPackageManagerState {
            packages: chord_packages,
        })?;

        Ok(())
    }

    pub fn get_package_by_name(&self, package_name: &str) -> Option<ChordPackage> {
        self.packages.read().get(package_name).cloned()
    }

    pub async fn load_package(&self, raw_chord_package: RawChordPackage) -> Result<ChordPackage> {
        let name = self.get_package_name(&raw_chord_package)?;
        log::debug!("loading package {}", name);

        let mut raw_chords_files = HashMap::new();
        let mut compiled_chords_files = HashMap::new();
        let mut global_chords = Vec::new();
        let mut parsed_chords_files = HashMap::new();

        for (path, contents) in raw_chord_package.chords_files_contents {
            let Ok(raw_chords_file) = toml::from_str::<RawChordsFile>(&contents).inspect_err(|e| {
                log::error!(
                    "error when loading package {}; failed to parse raw chords file {}:\n{}",
                    name,
                    e,
                    contents
                );
            }) else {
                continue;
            };

            raw_chords_files.insert(path.clone(), raw_chords_file);

            let Ok(parsed_chords_file) = contents.parse::<ParsedChordsFile>().inspect_err(|e| {
                log::error!(
                    "error when loading package {}; failed to parse chords file {}:\n{}",
                    name,
                    e,
                    contents
                );
            }) else {
                continue;
            };

            parsed_chords_files.insert(path, parsed_chords_file);
        }

        for (pathslug, parsed_chord_file) in &parsed_chords_files {
            let Ok(chords_file) = parsed_chord_file
                .compile(pathslug.to_owned(), &parsed_chords_files, &None)
                .inspect_err(|e| {
                    log::error!(
                        "skipping chords file {:?} in {} because of compilation error: {:?}",
                        pathslug,
                        name,
                        e
                    );
                })
            else {
                continue;
            };

            log::debug!(
                "compiled chords file {:#?} with {} chords",
                Path::new(&name).join(pathslug),
                chords_file.chords.len()
            );

            let is_bundled_chords_file = pathslug
                .components()
                .nth(1)
                .and_then(|c| c.as_os_str().to_str())
                .map(|s| s.starts_with('@'))
                .unwrap_or(false);
            if !is_bundled_chords_file {
                // We only want to add global chords from non-bundled chord files (i.e. pathslugs that
                // don't start with `chords/@`
                for chord in &chords_file.chords {
                    let first_char = chord.raw_trigger.chars().next();
                    let is_non_alphanumeric =
                        first_char.map(|c| !c.is_alphanumeric()).unwrap_or(false);

                    if is_non_alphanumeric {
                        global_chords.push(ChordReference {
                            package_name: name.clone(),
                            chords_file_pathslug: pathslug.clone(),
                            chord: chord.clone(),
                        });
                    }
                }
            }

            compiled_chords_files.insert(pathslug.to_owned(), chords_file);
        }

        let js_package = self
            .load_js_package(&name, &raw_chord_package.js_files_contents)
            .await?;

        let chord_package = ChordPackage {
            name: name.clone(),
            js_package,
            compiled_chords_files,
            raw_chords_files,
            global_chords,
        };

        self.packages.write().insert(name, chord_package.clone());

        Ok(chord_package)
    }

    async fn load_js_package(
        &self,
        package_name: &str,
        files: &HashMap<PathBuf, String>,
    ) -> Result<Option<ChordJsPackage>> {
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
                    let module =
                        Module::declare(ctx.clone(), module_specifier.clone(), js_string.clone())
                            .map_err(|e| {
                            anyhow::format_err!(
                                "error declaring module {:?}\nerror:{}\nfile:\n{}",
                                module_specifier,
                                format_js_error(&ctx, e),
                                format!("{}...", js_string.chars().take(100).collect::<String>())
                            )
                        })?;
                    let meta = module.meta()?;
                    meta.set("url", file_relpath.to_str())?;
                    let (_evaluated, promise) = module.eval()?;
                    promise.into_future::<()>().await?;
                    Ok(())
                })
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        }

        Ok(Some(ChordJsPackage::new(exported_files)))
    }

    fn get_package_name_from_package_json(
        &self,
        raw_chord_package: &RawChordPackage,
    ) -> anyhow::Result<Option<String>> {
        if let Some(package_json_contents) = &raw_chord_package.package_json_contents {
            let json: serde_json::Value = serde_json::from_str(package_json_contents)?;
            Ok(json
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()))
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
                if let Some(chords_file) = package.compiled_chords_files.get(&path) {
                    if chords_file
                        .chords
                        .iter()
                        .find(|c| c.trigger.matches(&input.keys))
                        .is_some()
                    {
                        return Some(package.clone());
                    }
                }
            }
        }

        for package in packages.values() {
            if package
                .global_chords
                .iter()
                .find(|c| c.chord.trigger.matches(&input.keys))
                .is_some()
            {
                return Some(package.clone());
            }
        }

        None
    }
}
