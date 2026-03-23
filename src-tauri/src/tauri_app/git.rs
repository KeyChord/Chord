use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::{Result, Context};
use observable_property::ObservableProperty;
use serde::Serialize;
use specta::Type;
use tauri::Wry;
use tauri_plugin_store::Store;
use typeshare::typeshare;
use crate::chords::{ChordPackage, LoadedAppChords};
use crate::feature::SafeAppHandle;
use crate::git::{clone_repo, GitHubRepoRef};
use crate::observables::{GitPackageRegistryObservable, GitPackageRegistryState, GitRepo};

pub const CHORD_SOURCES_STORE_PATH: &str = "chord-sources.json";
pub const LOCAL_FOLDERS_KEY: &str = "localFolders";

pub struct GitPackageRegistry {
    pub dir: PathBuf,
    pub observable: GitPackageRegistryObservable
}


fn discover_repos(repos_root: PathBuf) -> Result<Vec<GitRepo>> {
    if !repos_root.exists() {
        return Ok(Vec::new());
    }

    let mut repos = Vec::new();

    for owner_entry in fs::read_dir(&repos_root)? {
        let owner_entry = owner_entry?;
        let owner_path = owner_entry.path();
        if !owner_path.is_dir() {
            continue;
        }

        for repo_entry in fs::read_dir(&owner_path)? {
            let repo_entry = repo_entry?;
            let repo_path = repo_entry.path();
            if !repo_path.is_dir() || !repo_path.join(".git").exists() {
                continue;
            }

            let Some(owner) = owner_path.file_name().and_then(|segment| segment.to_str()) else {
                continue;
            };
            let Some(name) = repo_path.file_name().and_then(|segment| segment.to_str()) else {
                continue;
            };

            repos.push(
                GitHubRepoRef {
                    owner: owner.to_string(),
                    name: name.to_string(),
                }.into_repo(&repos_root),
            );
        }
    }

    repos.sort_by(|left, right| left.slug.cmp(&right.slug));
    Ok(repos)
}

impl GitPackageRegistry {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let dir = handle.path().app_cache_dir()?;
        let state = GitPackageRegistryState {
            git_repos: discover_repos(dir.clone())?
        };
        let observable = GitPackageRegistryObservable::new(handle.clone(), state)?;
        Ok(Self {
            dir,
            observable,
        })
    }

    pub fn github_repos_dir(&self) -> PathBuf {
        self.dir.join("repos/github.com")
    }

    pub fn add_repo(&self, repo_ref: GitHubRepoRef) -> Result<()> {
        let repos_root = self.github_repos_dir();
        let repo_path = repo_ref.local_path(&repos_root);
        let state = self.observable.get_state()?;
        let mut git_repos = state.git_repos.clone();

        let repo = if repo_path.join(".git").exists() {
            repo_ref.into_repo(&repos_root)
        } else {
            clone_repo(&repo_ref, &repo_path)?;
            repo_ref.into_repo(&repos_root)
        };
        git_repos.push(repo);

        self.observable.set_state(GitPackageRegistryState { git_repos })?;
        Ok(())
    }

    pub fn sync_repo(&self, repo_ref: GitHubRepoRef) -> Result<()> {
        let repos_root = self.github_repos_dir();
        let repo_path = repo_ref.local_path(&repos_root);

        if !repo_path.join(".git").exists() {
            anyhow::bail!("Repository {} has not been added yet", repo_ref.slug());
        }

        crate::git::refresh_repo(&repo_ref, &repo_path)?;
        let repo = repo_ref.into_repo(&repos_root);
        let state = self.observable.get_state()?;
        let mut git_repos = state.git_repos.clone();
        match git_repos.iter_mut().find(|r| r.owner == repo.owner && r.slug == r.owner) {
            Some(existing) => *existing = repo,
            None => git_repos.push(repo),
        }

        self.observable.set_state(GitPackageRegistryState { git_repos })?;
        Ok(())
    }

    pub fn load_all_packages(&self) -> anyhow::Result<Vec<ChordPackage>> {
        let mut packages = Vec::new();
        let state = self.observable.get_state()?;
        for repo in state.git_repos.iter() {
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

    pub fn load_repo_chords(&self, repo_input: &str) -> anyhow::Result<LoadedAppChords> {
        let repo_ref = GitHubRepoRef::parse(repo_input)?;
        let repos_root = self.github_repos_dir();
        let repo_path = repo_ref.local_path(&repos_root);

        if !repo_path.join(".git").exists() {
            anyhow::bail!("Repository {} has not been added yet", repo_ref.slug());
        }

        let repo =
            gix::open(&repo_path).context(format!("failed to open repo {}", repo_ref.slug()))?;
        let package = ChordPackage::load_from_git_repo(&repo)?;
        LoadedAppChords::from_folders(vec![package])
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

    pub fn load_chords(&self) -> anyhow::Result<LoadedAppChords> {
        LoadedAppChords::from_folders(vec![self.load()?])
    }
}

pub struct LocalPackageRegistry {
    safe_handle: SafeAppHandle
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
        let Some(folder_path) = self.safe_handle
            .dialog()
            .file()
            .set_title("Select Local Chord Folder")
            .blocking_pick_folder()
            .and_then(|folder_path| folder_path.into_path().ok())

        else {
            return Ok(None);
        };

        Ok(Some(self.package_from_user_input(&folder_path.display().to_string())?))
    }

    pub fn add(
        &self,
        folder_path: &str,
    ) -> Result<LocalChordPackage> {
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

    pub fn load_folder_chords(&self, folder_path: &str) -> anyhow::Result<LoadedAppChords> {
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
        self.safe_handle.store(CHORD_SOURCES_STORE_PATH)
            .context("failed to open chord sources store")
    }

    fn read_paths(&self) -> Result<Vec<String>> {
        let store = self.sources_store()?;
        let Some(value) = store.get(LOCAL_FOLDERS_KEY) else {
            return Ok(Vec::new());
        };

        serde_json::from_value(value).context("failed to parse local chord folder list")
    }

    fn write_paths(
        &self,
        paths: &[String],
    ) -> anyhow::Result<()> {
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
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        Ok(Self {
            git:GitPackageRegistry::new(handle.clone())?,
            local: LocalPackageRegistry::new(handle),
        })
    }

    pub fn load_all_chord_packages( &self ) -> Result<Vec<ChordPackage>> {
        let mut packages = vec![ChordPackage::load_bundled()?];
        packages.extend(self.git.load_all_packages()?);
        packages.extend(self.local.load_all_packages()?);
        Ok(packages)
    }
}
