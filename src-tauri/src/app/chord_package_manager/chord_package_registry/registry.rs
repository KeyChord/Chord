use super::ConfigPackageRegistry;
use super::LocalPackageRegistry;
use super::git::GitChordPackageRegistry;
use crate::app::state::AppSingleton;
use crate::models::RawChordPackage;
use crate::state::GitReposObservable;
use anyhow::Result;
use nject::injectable;
use std::collections::HashMap;
use tauri::AppHandle;

#[injectable]
pub struct ChordPackageRegistry {
    pub config: ConfigPackageRegistry,
    pub git: GitChordPackageRegistry,
    pub local: LocalPackageRegistry,
}

impl ChordPackageRegistry {
    /// TODO: return sorted by priority
    pub fn import_all_packages(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

        packages.extend(self.config.import_all_packages()?);
        packages.extend(self.git.import_all_packages()?);
        packages.extend(self.local.import_all_packages()?);

        Ok(packages)
    }
}
