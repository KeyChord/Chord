use crate::chords::{Chord, ChordPackage, ChordRegistry, GLOBAL_CHORD_RUNTIME_ID};
use crate::feature::SafeAppHandle;
use crate::git::GitHubRepoRef;
use crate::observables::{
    ActiveChordInfo, ChordRegistryObservable, GitReposObservable, Observable,
};
use anyhow::{Context, Result};
use serde::Serialize;
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Wry;
use tauri_plugin_store::Store;

pub const CHORD_SOURCES_STORE_PATH: &str = "chord-sources.json";
pub const LOCAL_FOLDERS_KEY: &str = "localFolders";

pub struct GitPackageRegistry {
    pub dir: PathBuf,

    handle: SafeAppHandle,
}

impl GitPackageRegistry {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let dir = handle.path().app_cache_dir()?;
        Ok(Self { dir, handle })
    }

    pub fn load_all_packages(&self) -> Result<Vec<ChordPackage>> {
        let mut packages = Vec::new();
        let state = self.handle.observable_state::<GitReposObservable>()?;
        for repo in state.repos.values() {
            match gix::open(&repo.local_path)
                .context(format!("failed to open repo {}", repo.slug))
                .and_then(|repo_handle| ChordPackage::load_from_git_repo(&repo_handle))
            {
                Ok(package) => packages.push(package),
                Err(error) => log::warn!("Skipping repo {}: {error}", repo.slug),
            }
        }

        Ok(packages)
    }

    pub fn load_repo_chords(&self, repo_input: &str) -> anyhow::Result<ChordRegistry> {
        let repo_ref = GitHubRepoRef::parse(repo_input)?;
        let repo_path = repo_ref.local_path(&self.dir);

        if !repo_path.join(".git").exists() {
            anyhow::bail!("Repository {} has not been added yet", repo_ref.slug());
        }

        let repo =
            gix::open(&repo_path).context(format!("failed to open repo {}", repo_ref.slug()))?;
        let package = ChordPackage::load_from_git_repo(&repo)?;
        ChordRegistry::new(vec![package])
    }
}

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

    pub fn load_chords(&self) -> anyhow::Result<ChordRegistry> {
        ChordRegistry::from_folders(vec![self.load()?])
    }
}

pub struct LocalPackageRegistry {
    safe_handle: SafeAppHandle,
}

impl LocalPackageRegistry {
    pub fn new(safe_handle: SafeAppHandle) -> Self {
        Self { safe_handle }
    }

    pub fn list(&self) -> Result<Vec<LocalChordPackage>> {
        let mut packages = self
            .read_paths()?
            .into_iter()
            .map(PathBuf::from)
            .map(LocalChordPackage::new)
            .collect::<Vec<_>>();

        packages.sort_by(|left, right| left.path().cmp(right.path()));
        Ok(packages)
    }

    pub fn pick(&self) -> Result<Option<LocalChordPackage>> {
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

    pub fn add(&self, folder_path: &str) -> Result<LocalChordPackage> {
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

    pub fn load_folder_chords(&self, folder_path: &str) -> anyhow::Result<ChordRegistry> {
        self.package_from_user_input(folder_path)?.load_chords()
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

    fn read_paths(&self) -> Result<Vec<String>> {
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

pub struct ChordPackageRegistry {
    pub git: GitPackageRegistry,
    pub local: LocalPackageRegistry,
}

impl ChordPackageRegistry {
    pub fn new_unloaded(handle: SafeAppHandle) -> Result<Self> {
        Ok(Self {
            git: GitPackageRegistry::new(handle.clone())?,
            local: LocalPackageRegistry::new(handle),
        })
    }

    pub fn load_all_chord_packages(&self) -> Result<Vec<ChordPackage>> {
        let mut packages = vec![ChordPackage::load_bundled()?];
        packages.extend(self.git.load_all_packages()?);
        packages.extend(self.local.load_all_packages()?);
        Ok(packages)
    }
}
