use std::collections::HashMap;
use crate::app::chord_package_registry::config::ConfigPackageRegistry;
use crate::app::state::StateSingleton;
use crate::models::RawChordPackage;
use tauri::AppHandle;
use crate::app::chord_package_registry::{GitChordPackageRegistry, LocalPackageRegistry};

pub struct ChordPackageRegistry {
    pub config: ConfigPackageRegistry,
    pub git: GitChordPackageRegistry,
    pub local: LocalPackageRegistry,
}

impl ChordPackageRegistry {
    pub fn new(handle: AppHandle) -> Self {
        Self {
            config: ConfigPackageRegistry::new(),
            git: GitChordPackageRegistry::new(handle.clone()),
            local: LocalPackageRegistry::new(handle.clone()),
        }
    }

    /// Returns sorted by priority
    pub fn import_all_packages(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

        packages.extend(self.config.import_all_packages()?);
        packages.extend(self.git.import_all_packages()?);
        packages.extend(self.local.import_all_packages()?);

        Ok(packages)
    }
}
