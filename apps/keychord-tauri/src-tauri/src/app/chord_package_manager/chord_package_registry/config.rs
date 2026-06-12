use crate::app::chord_package_manager::chord_package_registry::LocalPackageRegistry;
use crate::models::RawChordPackage;
use nject::injectable;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[injectable]
pub struct ConfigPackageRegistry;

impl ConfigPackageRegistry {
    pub fn import_all_packages(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

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

            if let Ok(package) =
                LocalPackageRegistry::import_from_local_folder(&path).inspect_err(|error| {
                    log::warn!(
                        "Error importing config package {}: {error}, skipping",
                        path.display()
                    );
                })
            {
                packages.insert(package.package_name(), package);
            }
        }

        Ok(packages)
    }

    fn get_packages_dir(&self) -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("chord").join("packages"))
    }
}
