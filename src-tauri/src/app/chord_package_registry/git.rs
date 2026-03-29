use crate::app::SafeAppHandle;
use crate::observables::GitReposObservable;
use anyhow::Result;
use std::path::PathBuf;
use crate::app::chord_package_registry::LocalPackageRegistry;
use crate::models::RawChordPackage;

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

    pub fn import_all_packages(&self) -> Result<Vec<RawChordPackage>> {
        let mut packages = Vec::new();
        let state = self.handle.observable_state::<GitReposObservable>()?;
        for repo in state.repos.values() {
            match LocalPackageRegistry::import_from_local_folder(repo.local_path.as_path()) {
                Ok(package) => packages.push(package),
                Err(error) => log::warn!("Skipping repo {}: {error}", repo.slug),
            }
        }

        Ok(packages)
    }
}
