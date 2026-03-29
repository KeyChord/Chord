use crate::app::SafeAppHandle;
use anyhow::Context;
use serde::Serialize;
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use fast_radix_trie::StringRadixMap;
use tauri::Wry;
use tauri_plugin_store::Store;
use walkdir::WalkDir;
use crate::models::RawChordPackage;

pub const CHORD_SOURCES_STORE_PATH: &str = "chord-sources.json";
pub const LOCAL_FOLDERS_KEY: &str = "localFolders";

#[derive(Serialize, Type)]
pub struct LocalChordPackage {
    path: PathBuf,
}

impl LocalChordPackage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

}

pub struct LocalPackageRegistry {
    safe_handle: SafeAppHandle,
}

impl LocalPackageRegistry {
    pub fn new(safe_handle: SafeAppHandle) -> Self {
        Self { safe_handle }
    }

    pub fn list_package_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut packages = self
            .read_paths()?
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        packages.sort_by(|left, right| left.cmp(right));
        Ok(packages)
    }

    pub fn pick(&self) -> anyhow::Result<Option<LocalChordPackage>> {
        let Some(folder_path) = self
            .safe_handle
            .dialog()
            .file()
            .set_title("Select Local Chord Folder")
            .blocking_pick_folder()
            .and_then(|folder_path| folder_path.into_path().ok())
        else {
            return Ok(None);
        };

        Ok(Some(self.package_from_user_input(
            &folder_path.display().to_string(),
        )?))
    }

    pub fn add(&self, folder_path: &str) -> anyhow::Result<LocalChordPackage> {
        let package = self.package_from_user_input(folder_path)?;
        let canonical_path = package.path().display().to_string();
        let mut paths = self.read_paths()?;

        if !paths.contains(&canonical_path) {
            paths.push(canonical_path);
            paths.sort();
            self.write_paths(paths.as_slice())?;
        }

        Ok(package)
    }

    pub fn import_all_packages(&self) -> anyhow::Result<Vec<RawChordPackage>> {
        let mut packages = Vec::new();

        for local_package_path in self.list_package_paths()? {
            match Self::import_from_local_folder(local_package_path.as_path()) {
                Ok(package) => packages.push(package),
                Err(error) => {
                    log::warn!(
                        "Error importing local folder {}: {error}, skipping",
                        local_package_path.display()
                    );
                }
            }
        }

        Ok(packages)
    }

    fn package_from_user_input(&self, path: &str) -> anyhow::Result<LocalChordPackage> {
        Ok(LocalChordPackage::new(self.canonicalize_path(path)?))
    }

    fn sources_store(&self) -> anyhow::Result<Arc<Store<Wry>>> {
        self.safe_handle
            .store(CHORD_SOURCES_STORE_PATH)
            .context("failed to open chord sources store")
    }

    fn read_paths(&self) -> anyhow::Result<Vec<String>> {
        let store = self.sources_store()?;
        let Some(value) = store.get(LOCAL_FOLDERS_KEY) else {
            return Ok(Vec::new());
        };

        serde_json::from_value(value).context("failed to parse local chord folder list")
    }

    fn write_paths(&self, paths: &[String]) -> anyhow::Result<()> {
        let store = self.sources_store()?;
        let value =
            serde_json::to_value(paths).context("failed to serialize local chord folder list")?;
        store.set(LOCAL_FOLDERS_KEY, value);
        Ok(())
    }

    fn canonicalize_path(&self, path: &str) -> anyhow::Result<PathBuf> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Folder path cannot be empty");
        }

        let canonical_path =
            fs::canonicalize(trimmed).context(format!("failed to access folder {trimmed}"))?;
        if !canonical_path.is_dir() {
            anyhow::bail!("{trimmed} is not a folder");
        }

        Ok(canonical_path)
    }

    pub fn import_from_local_folder(root: &Path) -> anyhow::Result<RawChordPackage> {
        let mut chords_files_contents = StringRadixMap::new();
        let mut js_files_contents = StringRadixMap::new();
        let mut bin_files_contents = StringRadixMap::new();

        let dirname = root.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let package_json_contents = fs::read_to_string(root.join("package.json")).ok();

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
                            chords_files_contents.insert(key, content);
                        }
                    }
                    "js" => {
                        let content = fs::read_to_string(path)?;
                        js_files_contents.insert(key, content);
                    }
                    "bin" => {
                        let content = fs::read(path)?;
                        bin_files_contents.insert(key, content);
                    }
                    _ => {}
                }
            }
        }

        Ok(RawChordPackage {
            dirname,
            package_json_contents,
            chords_files_contents,
            js_files_contents,
            bin_files_contents,
        })
    }
}

fn is_supported_macos_chord_filename(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    file_name == "macos.toml" || file_name.ends_with(".macos.toml")
}
