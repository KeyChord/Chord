use crate::app::AppHandleExt;
use crate::observables::GitReposObservable;
use anyhow::Result;
use tauri::AppHandle;
use crate::app::chord_package_manager::local::LocalPackageRegistry;
use crate::app::state::StateSingleton;
use crate::models::RawChordPackage;

pub struct GitChordPackageRegistry {
    handle: AppHandle,
}

impl StateSingleton for GitChordPackageRegistry {
    fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl GitChordPackageRegistry {
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
