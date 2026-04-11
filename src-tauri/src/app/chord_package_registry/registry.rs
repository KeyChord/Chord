use crate::app::chord_package_registry::config::ConfigPackageRegistry;
use crate::app::chord_package_registry::{GitChordPackageRegistry, LocalPackageRegistry};
use crate::app::state::AppSingleton;
use crate::models::RawChordPackage;
use std::collections::HashMap;
use tauri::AppHandle;

pub struct ChordPackageRegistry {
    pub config: ConfigPackageRegistry,
    pub git: GitChordPackageRegistry,
    pub local: LocalPackageRegistry,
}

impl ChordPackageRegistry {
    /// Returns sorted by priority
    pub fn import_all_packages(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

        packages.extend(self.config.import_all_packages()?);
        packages.extend(self.git.import_all_packages()?);
        packages.extend(self.local.import_all_packages()?);

        Ok(packages)
    }
}
