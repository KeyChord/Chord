use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::Context;
use serde::Serialize;
use specta::Type;
use tauri::Wry;
use tauri_plugin_store::Store;
use crate::app::SafeAppHandle;
use crate::chords::ChordPackage;
use crate::registry::{CHORD_SOURCES_STORE_PATH, LOCAL_FOLDERS_KEY};

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

    pub fn load(&self) -> anyhow::Result<ChordPackage> {
        ChordPackage::load_from_local_folder(&self.path)
    }
}

pub struct LocalPackageRegistry {
    safe_handle: SafeAppHandle,
}

impl LocalPackageRegistry {
    pub fn new(safe_handle: SafeAppHandle) -> Self {
        Self { safe_handle }
    }

    pub fn list(&self) -> anyhow::Result<Vec<LocalChordPackage>> {
        let mut packages = self
            .read_paths()?
            .into_iter()
            .map(PathBuf::from)
            .map(LocalChordPackage::new)
            .collect::<Vec<_>>();

        packages.sort_by(|left, right| left.path().cmp(right.path()));
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

    pub fn load_all_packages(&self) -> anyhow::Result<Vec<ChordPackage>> {
        let mut packages = Vec::new();

        for local_package in self.list()? {
            match local_package.load() {
                Ok(package) => packages.push(package),
                Err(error) => {
                    log::warn!(
                        "Skipping local folder {}: {error}",
                        local_package.path().display()
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
}

