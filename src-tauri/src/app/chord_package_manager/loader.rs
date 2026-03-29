use std::fs;
use std::path::Path;
use fast_radix_trie::StringRadixMap;
use walkdir::WalkDir;

struct ChordPackageLoader {}

impl ChordPackageLoader {

    pub fn load_from_local_git_repo(repo: &gix::Repository) -> anyhow::Result<ChordPackage> {
        let root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

        Self::load_from_local_folder(root)
    }

    pub fn load_from_local_folder(root: &Path) -> anyhow::Result<ChordPackage> {
        let mut chords_files = StringRadixMap::new();
        let mut js_files = StringRadixMap::new();
        let mut bin_files = StringRadixMap::new();

        // --- package name ---
        let name = {
            let pkg_json_path = root.join("package.json");

            if pkg_json_path.exists() {
                fs::read_to_string(&pkg_json_path)
                    .ok()
                    .and_then(|content| {
                        serde_json::from_str::<serde_json::Value>(&content).ok()
                    })
                    .and_then(|json| json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
            } else {
                None
            }
        }
            .or_else(|| {
                root.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("Could not determine package name"))?;

        // --- walk only specific dirs ---
        for dir in ["chords", "js", "bin"] {
            let dir_path = root.join(dir);

            if !dir_path.exists() {
                continue;
            }

            for entry in WalkDir::new(&dir_path) {
                let entry = entry?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let relative_path = path.strip_prefix(root)?;
                let key = relative_path.to_string_lossy().to_string();

                match dir {
                    "chords" => {
                        if is_supported_macos_chord_filename(relative_path) {
                            let content = fs::read_to_string(path)?;

                            match AppChordsFile::parse(&content) {
                                Ok(parsed) => {
                                    chords_files.insert(key, parsed);
                                }
                                Err(error) => {
                                    log::warn!("Skipping invalid {:?}: {}", path, error);
                                }
                            }
                        }
                    }
                    "js" => {
                        let content = fs::read_to_string(path)?;
                        js_files.insert(key, content);
                    }
                    "bin" => {
                        let content = fs::read(path)?;
                        bin_files.insert(key, content);
                    }
                    _ => {}
                }
            }
        }

        Ok(ChordPackage {
            name,
            chords_files,
            js_files,
            bin_files,
        })
    }
}

fn is_supported_macos_chord_filename(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    file_name == "macos.toml" || file_name.ends_with(".macos.toml")
}
