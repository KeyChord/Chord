use std::collections::HashMap;
use std::sync::RwLock;
use crate::app::chord_js_package_registry::ChordJsPackage;
use crate::app::SafeAppHandle;
use crate::models::{ChordInput, ChordPackage, RawChordPackage};
use crate::models::chords_file::ChordsFile;
use crate::input::Key;
use fast_radix_trie::StringRadixMap;

pub mod chord_package;


pub struct ChordPackageManager {
    packages: RwLock<HashMap<String, ChordPackage>>,

    handle: SafeAppHandle,
}


impl ChordPackageManager {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle, packages: RwLock::new(HashMap::new()) }
    }

    pub async fn load_package(&self, raw_chord_package: RawChordPackage) -> anyhow::Result<ChordPackage> {
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

            if path.starts_with("chords/") && path.ends_with("/macos.toml") {
                let bundle_id = &path[7..path.len() - 11];
                let bundle_id = bundle_id.replace('/', ".");
                app_chords_files.insert(bundle_id, chords_file);
            } else if path == "chords/macos.toml" {
                // If it's directly under chords/, we can treat it as a special case or ignore if it needs a bundle ID
                // For now, let's assume it might be for a 'global' context or similar if it has no bundle ID path
                app_chords_files.insert("".to_string(), chords_file);
            }
        }

        let mut exported_files = StringRadixMap::new();
        for (path, js) in raw_chord_package.js_files_contents {
            exported_files.insert(path, js);
        }

        let js_package = if !exported_files.is_empty() {
            Some(ChordJsPackage::new(exported_files))
        } else {
            None
        };

        let chord_package = ChordPackage {
            name: name.clone(),
            js_package,
            app_chords_files,
            global_chords
        };

        self.packages.write().unwrap().insert(name, chord_package.clone());

        Ok(chord_package)
    }

    fn get_package_name_from_package_json(&self, raw_chord_package: &RawChordPackage) -> anyhow::Result<Option<String>> {
        if let Some(package_json_contents) = &raw_chord_package.package_json_contents {
            let json: serde_json::Value = serde_json::from_str(package_json_contents)?;
            Ok(json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }

    fn get_package_name(&self, raw_chord_package: &RawChordPackage) -> anyhow::Result<String> {
        if let Some(name) = self.get_package_name_from_package_json(raw_chord_package)? {
            Ok(name)
        } else {
            Ok(raw_chord_package.dirname.clone())
        }
    }

    pub fn resolve_package_for_input(&self, input: &ChordInput) -> Option<ChordPackage> {
        let packages = self.packages.read().unwrap();

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
