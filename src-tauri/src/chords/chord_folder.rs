use crate::chords::AppChordsFile;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use std::collections::HashMap;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ChordFolder {
    // Map from file path to chord
    pub chords_files: HashMap<String, AppChordsFile>,
    pub lua_files: HashMap<String, String>,
}

static BUNDLED_MACOS_CHORDS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../data/chords/macos");

impl ChordFolder {
    pub fn load_bundled() -> Result<Self> {
        let mut chords_files = HashMap::new();
        for file in BUNDLED_MACOS_CHORDS_DIR.find("**/chords.toml")? {
            let path = file.path().to_string_lossy().to_string();
            let content = file
                .as_file()
                .and_then(|f| f.contents_utf8())
                .ok_or_else(|| anyhow::anyhow!("Could not read file as utf8: {:?}", file.path()))?;
            let app_chords_file = AppChordsFile::parse(content)?;
            chords_files.insert(path, app_chords_file);
        }

        Ok(Self { chords_files, lua_files: HashMap::new() })
    }

    pub fn load_from_git_repo(repo: &gix::Repository) -> Result<Self> {
        let mut chords_files = HashMap::new();
        let mut lua_files = HashMap::new();

        let root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

        // ------------------------
        // Load chords/macos
        // ------------------------
        let chords_dir = root.join("chords").join("macos");
        if chords_dir.exists() {
            for entry in WalkDir::new(&chords_dir) {
                let entry = entry?;
                if entry.file_name() == "chords.toml" {
                    let content = std::fs::read_to_string(entry.path())?;
                    match AppChordsFile::parse(&content) {
                        Ok(parsed) => {
                            let relative_path = entry
                                .path()
                                .strip_prefix(&chords_dir)?
                                .to_string_lossy()
                                .to_string();

                            chords_files.insert(relative_path, parsed);
                        }
                        Err(error) => {
                            log::warn!("Skipping invalid {:?}: {}", entry.path(), error);
                            continue;
                        }
                    };
                }
            }
        } else {
            log::debug!("No chords/macos folder found in {:?}", root);
        }

        // ------------------------
        // Load lua/
        // ------------------------
        let lua_dir = root.join("lua");
        if lua_dir.exists() {
            for entry in WalkDir::new(&lua_dir) {
                let entry = entry?;

                if entry.file_type().is_file() {
                    let path = entry.path();

                    // optional: only include .lua files
                    if path.extension().and_then(|e| e.to_str()) == Some("lua") {
                        match std::fs::read_to_string(path) {
                            Ok(content) => {
                                let relative_path = path
                                    .strip_prefix(&lua_dir)?
                                    .to_string_lossy()
                                    .to_string();

                                lua_files.insert(relative_path, content);
                            }
                            Err(err) => {
                                log::warn!("Skipping lua file {:?}: {}", path, err);
                            }
                        }
                    }
                }
            }
        } else {
            log::debug!("No lua folder found in {:?}", root);
        }

        Ok(Self {
            chords_files,
            lua_files,
        })
    }

    pub fn merge(&mut self, other: Self) {
        self.chords_files.extend(other.chords_files);
        self.lua_files.extend(other.lua_files);
    }
}
