use std::collections::HashMap;
use std::sync::RwLock;
use crate::app::chord_js_package_registry::ChordJsPackage;
use crate::app::SafeAppHandle;
use crate::models::{ChordPackage, RawChordPackage};
use crate::models::chords_file::ChordsFile;
use fast_radix_trie::StringRadixMap;

mod package_loader;
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

            if path == "chords/global.toml" {
                for chord in chords_file.chords {
                    global_chords.insert(chord.string.clone(), chord);
                }
            } else if path.starts_with("chords/") && path.ends_with(".toml") {
                let bundle_id = &path[7..path.len()-5];
                app_chords_files.insert(bundle_id.to_string(), chords_file);
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
}
