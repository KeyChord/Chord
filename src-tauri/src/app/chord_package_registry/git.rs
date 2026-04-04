use crate::app::AppHandleExt;
use crate::app::chord_package_registry::LocalPackageRegistry;
use crate::app::state::StateSingleton;
use crate::models::RawChordPackage;
use crate::observables::GitReposObservable;
use anyhow::Result;
use std::collections::HashMap;
use tauri::AppHandle;

pub struct GitChordPackageRegistry {
    handle: AppHandle,
}

impl StateSingleton for GitChordPackageRegistry {
    fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl GitChordPackageRegistry {
    pub fn import_all_packages(&self) -> Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();
        let state = self.handle.observable_state::<GitReposObservable>()?;
        for repo in state.repos.values() {
            if let Ok(package) =
                LocalPackageRegistry::import_from_local_folder(repo.local_abspath.as_path())
                    .inspect_err(|e| {
                        log::warn!("skipping repo {} because of import error: {e}", repo.slug)
                    })
            {
                packages.insert(package.package_name(), package);
            }
        }

        Ok(packages)
    }
}
