use crate::app::chord_package_manager::local::LocalPackageRegistry;
use crate::models::RawChordPackage;
use std::fs;
use std::path::PathBuf;

pub struct ConfigPackageRegistry;

impl ConfigPackageRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn import_all_packages(&self) -> anyhow::Result<Vec<RawChordPackage>> {
        let mut packages = Vec::new();

        let Some(packages_dir) = self.get_packages_dir() else {
            return Ok(packages);
        };

        if !packages_dir.exists() {
            return Ok(packages);
        }

        for entry in fs::read_dir(&packages_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            match LocalPackageRegistry::import_from_local_folder(&path) {
                Ok(package) => packages.push(package),
                Err(error) => {
                    log::warn!(
                        "Error importing config package {}: {error}, skipping",
                        path.display()
                    );
                }
            }
        }

        Ok(packages)
    }

    fn get_packages_dir(&self) -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("chord").join("packages"))
    }
}

impl Default for ConfigPackageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
