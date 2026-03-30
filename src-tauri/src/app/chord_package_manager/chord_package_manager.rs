use std::collections::HashMap;
use std::sync::Arc;
use fast_radix_trie::StringRadixMap;
use llrt_core::Module;
use crate::app::chord_package_manager::{ChordJsPackage, ChordPackage};
use crate::app::chord_package_manager::registry::ChordPackageRegistry;
use crate::input::Key;
use crate::models::{ChordInput, ChordsFile, RawChordPackage};
use crate::quickjs::{format_js_error, with_js};
use anyhow::Result;
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
            observable: ChordPackageManagerObservable::none(),
            handle
        }
    }
}

impl ChordPackageManager {
    pub fn init(&mut self, observable: ChordPackageManagerObservable) -> Result<()> {
        self.observable = observable;
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
        let mut app_chords_files = HashMap::new();
        let mut global_chords = HashMap::new();

        for (path, contents) in raw_chord_package.chords_files_contents {
            let mut chords_file = ChordsFile::parse(&contents);
            chords_file.relpath = path.clone();

            for chord in &chords_file.chords {
                let first_char = chord.string_key.chars().next();
                let is_non_alphanumeric = first_char.map(|c| !c.is_alphanumeric()).unwrap_or(false);

                if is_non_alphanumeric {
                    global_chords.insert(chord.string_key.clone(), chord.clone());
                }
            }

            if let Some(inner) = path.strip_prefix("chords/").and_then(|p| p.strip_suffix("/macos.toml")) {
                let bundle_id = inner.replace('/', ".");
                app_chords_files.insert(bundle_id, chords_file);
            } else {
                app_chords_files.insert("/global".to_string(), chords_file);
            }
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

    async fn load_js_package(&self, package_name: &str, files: &StringRadixMap<String>) -> Result<Option<ChordJsPackage>> {
        if files.is_empty() {
            return Ok(None)     ;
        }

        let mut exported_files = HashMap::new();

        for (file_relpath, js) in files.iter() {
            let package_name_bytes = package_name.as_bytes().to_vec();
            exported_files.insert(file_relpath.clone(), js.clone());
            let js_string = js.clone();
            with_js(self.handle.clone(), move |ctx| {
                Box::pin(async move {
                    let module = Module::declare(ctx.clone(), package_name_bytes.clone(), js_string.clone()).map_err(
                        |e| anyhow::format_err!(
                            "error declaring module {:?}\nerror:{}\nfile:\n{}",
                            package_name_bytes.clone().into_string(),
                            format_js_error(&ctx, e),
                            js_string.clone()
                        )
                    )?;
                    let meta = module.meta()?;
                    meta.set("url", file_relpath)?;
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
            for package in packages.values() {
                if package.app_chords_files.contains_key(app_id) {
                    return Some(package.clone());
                }
            }
        }

        let sequence_str = Key::serialize_sequence(&input.keys)?;
        for package in packages.values() {
            if package.global_chords.contains_key(&sequence_str) {
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
