use super::PackageStore;
use crate::models::RawChordPackage;
use nject::injectable;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[injectable]
pub struct ConfigPackageRegistry {
    handle: AppHandle,
}

impl ConfigPackageRegistry {
    pub fn import_all_packages(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

        let store = PackageStore::for_app(&self.handle)?;
        let packages_dir = store.migrate_config(&self.get_packages_dir()?)?;

        if !packages_dir.exists() {
            return Ok(packages);
        }

        for entry in fs::read_dir(&packages_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            if let Ok(package) = store.import_package(&path).inspect_err(|error| {
                log::warn!(
                    "Error importing config package {}: {error}, skipping",
                    path.display()
                );
            }) {
                packages.insert(package.package_name(), package);
            }
        }

        Ok(packages)
    }

    fn get_packages_dir(&self) -> anyhow::Result<PathBuf> {
        Ok(self.handle.path().app_config_dir()?.join("packages"))
    }
}
