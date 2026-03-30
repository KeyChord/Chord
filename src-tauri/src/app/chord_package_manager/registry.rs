use tauri::AppHandle;
use crate::app::chord_package_manager::git::GitChordPackageRegistry;
use crate::app::chord_package_manager::local::LocalPackageRegistry;
use crate::app::state::StateSingleton;
use crate::models::RawChordPackage;

pub struct ChordPackageRegistry {
    pub git: GitChordPackageRegistry,
    pub local: LocalPackageRegistry,
}

impl ChordPackageRegistry {
    pub fn new(handle: AppHandle) -> Self {
        Self {
            git: GitChordPackageRegistry::new(handle.clone()),
            local: LocalPackageRegistry::new(handle.clone()),
        }
    }

    pub fn import_all_packages(&self) -> anyhow::Result<Vec<RawChordPackage>> {
        let mut packages = Vec::new();
        packages.extend(self.git.import_all_packages()?);
        packages.extend(self.local.import_all_packages()?);
        Ok(packages)
    }
}
