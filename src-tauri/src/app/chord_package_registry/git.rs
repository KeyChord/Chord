use crate::app::SafeAppHandle;
use crate::app::chord_package::ChordPackage;
use crate::observables::GitReposObservable;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct GitChordPackageRegistry {
    #[allow(dead_code)]
    pub dir: PathBuf,

    handle: SafeAppHandle,
}

impl GitChordPackageRegistry {
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
}
