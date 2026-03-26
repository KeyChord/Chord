use crate::app::chord_package::AppChordsFile;
use anyhow::Result;
use fast_radix_trie::StringRadixMap;
use include_dir::{Dir, include_dir};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ChordPackage {
    pub root_dir: Option<PathBuf>,

    // Map from file path to chord
    pub chords_files: StringRadixMap<AppChordsFile>,
    pub js_files: StringRadixMap<String>,
}

static BUNDLED_MACOS_CHORDS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../data/chords");

impl ChordPackage {
    pub fn load_bundled() -> Result<Self> {
        let mut js_files = StringRadixMap::new();
        let mut chords_files = StringRadixMap::new();
        let mut chord_file_paths = Vec::new();

        for entry in BUNDLED_MACOS_CHORDS_DIR.find("**/*")? {
            let Some(file) = entry.as_file() else {
                continue;
            };

            let relative_path = entry.path().to_path_buf();

            if relative_path.starts_with("chords")
                && is_supported_macos_chord_filename(&relative_path)
            {
                chord_file_paths.push(relative_path.clone());
            }

            if relative_path.extension().and_then(|value| value.to_str()) == Some("js") {
                let content = file.contents_utf8().ok_or_else(|| {
                    anyhow::anyhow!("Could not read file as utf8: {:?}", entry.path())
                })?;

                js_files.insert(relative_path.to_string_lossy().to_string(), content.to_string());
            }
        }

        chord_file_paths.sort();
        for relative_path in chord_file_paths {
            let file = BUNDLED_MACOS_CHORDS_DIR.get_file(&relative_path).ok_or_else(|| {
                anyhow::anyhow!("Could not find bundled file: {:?}", relative_path)
            })?;
            let content = file.contents_utf8().ok_or_else(|| {
                anyhow::anyhow!("Could not read file as utf8: {:?}", relative_path)
            })?;

            match AppChordsFile::parse(content) {
                Ok(parsed) => {
                    chords_files.insert(relative_path.to_string_lossy().to_string(), parsed);
                }
                Err(error) => {
                    log::warn!("Skipping invalid {:?}: {}", relative_path, error);
                }
            }
        }

        Ok(Self {
            root_dir: None,
            chords_files,
            js_files,
        })
    }

    pub fn load_from_git_repo(repo: &gix::Repository) -> Result<Self> {
        let root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

        Self::load_from_local_folder(root)
    }

    pub fn load_from_local_folder(root: &Path) -> Result<Self> {
        let mut js_files = StringRadixMap::new();
        let mut chords_files = StringRadixMap::new();
        let mut chord_file_paths = Vec::new();

        if root.exists() {
            for entry in WalkDir::new(root) {
                let entry = entry?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let relative_path = path.strip_prefix(root)?.to_path_buf();

                if relative_path.starts_with("chords")
                    && is_supported_macos_chord_filename(&relative_path)
                {
                    chord_file_paths.push(relative_path.clone());
                }

                // ------------------------
                // Handle *.js files
                // ------------------------
                if path.extension().and_then(|s| s.to_str()) == Some("js") {
                    let content = std::fs::read_to_string(path)?;

                    js_files.insert(relative_path.to_string_lossy().to_string(), content);
                }
            }

            chord_file_paths.sort();
            for relative_path in chord_file_paths {
                let path = root.join(&relative_path);
                let content = std::fs::read_to_string(&path)?;

                match AppChordsFile::parse(&content) {
                    Ok(parsed) => {
                        chords_files.insert(relative_path.to_string_lossy().to_string(), parsed);
                    }
                    Err(error) => {
                        log::warn!("Skipping invalid {:?}: {}", path, error);
                    }
                }
            }
        } else {
            log::debug!("Root folder does not exist: {:?}", root);
        }

        Ok(Self {
            root_dir: Some(root.to_path_buf()),
            chords_files,
            js_files,
        })
    }

    #[allow(dead_code)]
    pub fn merge(&mut self, other: Self) {
        self.chords_files.extend(other.chords_files);
        self.js_files.extend(other.js_files);
    }
}

fn is_supported_macos_chord_filename(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    file_name == "macos.toml" || file_name.ends_with(".macos.toml")
}
