use super::ConfigPackageRegistry;
use super::LocalPackageRegistry;
use super::{PersonalPackageRegistry, PersonalPackageState};
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
    pub personal: PersonalPackageRegistry,
}

pub struct ImportedPackages {
    pub packages: HashMap<String, RawChordPackage>,
    /// The folders in `~/.chord/packages`, in priority order.
    pub personal: Vec<PersonalPackageState>,
}

impl ChordPackageRegistry {
    /// Default config packages have the lowest priority, followed by GitHub, local sources, and
    /// finally the user's personal packages in `~/.chord/packages`.
    pub fn import_all_packages(&self) -> anyhow::Result<ImportedPackages> {
        let mut packages = HashMap::new();

        packages.extend(self.config.import_all_packages()?);
        packages.extend(self.git.import_packages(false)?);
        packages.extend(self.local.import_monorepos()?);
        packages.extend(self.git.import_packages(true)?);
        packages.extend(self.local.import_all_packages()?);
        let (personal_packages, personal) = self.personal.import_packages()?;
        packages.extend(personal_packages);

        super::core::protect_core_package(&mut packages)?;

        Ok(ImportedPackages { packages, personal })
    }
}
