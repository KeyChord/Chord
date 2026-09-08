use crate::app::AppHandleExt;
use crate::app::state::AppSingleton;
use crate::models::RawChordPackage;
use anyhow::Context;
use nject::injectable;
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::{Store, StoreExt};
use walkdir::WalkDir;

pub const CHORD_SOURCES_STORE_PATH: &str = "chord-sources.json";
pub const LOCAL_FOLDERS_KEY: &str = "localFolders";
pub const LOCAL_MONOREPOS_KEY: &str = "localMonorepos";

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

#[injectable]
pub struct LocalPackageRegistry {
    handle: AppHandle,
}

impl LocalPackageRegistry {
    pub fn monorepo_selections(&self) -> anyhow::Result<HashMap<String, Vec<String>>> {
        self.sources_store()?
            .get("monorepoSelections")
            .map(serde_json::from_value)
            .transpose()
            .map(|value| value.unwrap_or_default())
            .context("Failed to read monorepo package selections")
    }

    pub fn monorepo_root(&self, source: &str) -> anyhow::Result<PathBuf> {
        if self.list_monorepos()?.iter().any(|path| path == source) {
            return Ok(PathBuf::from(source));
        }
        let store = self
            .handle
            .app_state()
            .chord_package_manager()
            .registry
            .git
            .git_repos_store
            .store()?;
        let repos = super::git::load_repos(&store)?;
        let repo = repos
            .get(source)
            .filter(|repo| repo.is_monorepo)
            .context("Monorepo is not linked")?;
        Ok(repo
            .linked_local_path
            .as_ref()
            .unwrap_or(&repo.local_abspath)
            .clone())
    }

    pub fn set_monorepo_packages(&self, source: &str, names: Vec<String>) -> anyhow::Result<()> {
        let root = self.monorepo_root(source)?;
        let available = super::import_monorepo_packages(&root)?;
        anyhow::ensure!(
            names.iter().all(|name| available.contains_key(name)),
            "Unknown monorepo package"
        );
        let mut selections = self.monorepo_selections()?;
        selections.insert(source.to_owned(), names);
        let store = self.sources_store()?;
        store.set("monorepoSelections", serde_json::to_value(selections)?);
        store.save()?;
        Ok(())
    }

    pub fn list_monorepos(&self) -> anyhow::Result<Vec<String>> {
        let value = self.sources_store()?.get(LOCAL_MONOREPOS_KEY);
        let mut paths: Vec<String> = value
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        paths.sort();
        Ok(paths)
    }

    pub fn add_monorepo(&self, path: &str) -> anyhow::Result<()> {
        let root = super::validate_monorepo_path(path)?.display().to_string();
        let mut paths = self.list_monorepos()?;
        if !paths.contains(&root) {
            paths.push(root);
            paths.sort();
            self.write_monorepos(&paths)?;
        }
        Ok(())
    }

    pub fn remove_monorepo(&self, path: &str) -> anyhow::Result<()> {
        // Use the saved path so a missing/moved checkout can still be unlinked.
        let mut paths = self.list_monorepos()?;
        paths.retain(|saved| saved != path);
        self.write_monorepos(&paths)
    }

    fn write_monorepos(&self, paths: &[String]) -> anyhow::Result<()> {
        let store = self.sources_store()?;
        store.set(LOCAL_MONOREPOS_KEY, serde_json::to_value(paths)?);
        store.save()?;
        Ok(())
    }

    pub fn import_monorepos(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();
        let selections = self.monorepo_selections()?;
        for path in self.list_monorepos()? {
            packages.extend(super::import_selected_monorepo_packages(
                Path::new(&path),
                selections.get(&path).map(Vec::as_slice).unwrap_or_default(),
            )?);
        }
        Ok(packages)
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
            .handle
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

    pub fn import_all_packages(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

        for local_package_path in self.list_package_paths()? {
            if let Ok(package) = Self::import_from_local_folder(local_package_path.as_path())
                .inspect_err(|e| {
                    log::warn!(
                        "Error importing local folder {}: {e}, skipping",
                        local_package_path.display()
                    );
                })
            {
                packages.insert(package.package_name(), package);
            }
        }

        Ok(packages)
    }

    fn package_from_user_input(&self, path: &str) -> anyhow::Result<LocalChordPackage> {
        Ok(LocalChordPackage::new(self.canonicalize_path(path)?))
    }

    fn sources_store(&self) -> anyhow::Result<Arc<Store<Wry>>> {
        self.handle
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
        let mut chords_files_contents = HashMap::new();
        let mut js_files_contents = HashMap::new();
        let mut bin_files_contents = HashMap::new();

        let dirname = root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let package_json_contents = fs::read_to_string(root.join("package.json")).ok();

        // Native modules under `target/` are not read: the package's own JS loads them from
        // disk with `process.dlopen` (see the `chord` module's `resolveNativeModulePath`).
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

                let relpath = path.strip_prefix(root)?;

                match dir {
                    "chords" => {
                        let content = fs::read_to_string(path)?;
                        chords_files_contents.insert(relpath.to_path_buf(), content);
                    }
                    "js" => {
                        if path.extension().is_some_and(|ext| ext == "js") {
                            let content = fs::read_to_string(path)?;
                            js_files_contents.insert(relpath.to_path_buf(), content);
                        }
                    }
                    "bin" => {
                        let content = fs::read(path)?;
                        bin_files_contents.insert(relpath.to_path_buf(), content);
                    }
                    _ => {}
                }
            }
        }

        log::debug!(
            "loaded chord package from {:?}:\njs: {:?}\nchords: {:?}\nbin: {:?}",
            root,
            js_files_contents.keys(),
            chords_files_contents.keys(),
            bin_files_contents.keys()
        );

        Ok(RawChordPackage {
            dirname,
            root: root.to_path_buf(),
            package_json_contents,
            chords_files_contents,
            js_files_contents,
            bin_files_contents,
        })
    }
}

#[allow(dead_code)]
fn is_supported_macos_chord_filename(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    file_name == "macos.toml" || file_name.ends_with(".macos.toml")
}
