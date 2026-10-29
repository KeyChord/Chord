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
    pub trust: super::PackageTrustRegistry,
}

pub struct ImportedPackages {
    pub packages: HashMap<String, RawChordPackage>,
    /// The folders in `~/.chord/packages`, in priority order.
    pub personal: Vec<PersonalPackageState>,
    pub third_party: Vec<super::ThirdPartyPackage>,
}

impl ChordPackageRegistry {
    /// Default config packages have the lowest priority, followed by GitHub, local sources, and
    /// finally the user's personal packages in `~/.chord/packages`.
    pub fn import_all_packages(&self) -> anyhow::Result<ImportedPackages> {
        let mut packages = HashMap::new();
        let store = self.local.package_store()?;
        let approvals = self.trust.approvals()?;
        let mut third_party = Vec::new();
        let mut merge = |mut source| -> anyhow::Result<()> {
            third_party.extend(super::trust::filter_packages(&store, &mut source, &approvals)?);
            packages.extend(source);
            Ok(())
        };
        merge(self.config.import_all_packages()?)?;
        merge(self.git.import_packages(false)?)?;
        merge(self.local.import_monorepos()?)?;
        merge(self.git.import_packages(true)?)?;
        merge(self.local.import_all_packages()?)?;
        let (personal_packages, personal) = self.personal.import_packages()?;
        merge(personal_packages)?;
        third_party.sort_by(|a, b| a.path.cmp(&b.path));
        third_party.dedup_by(|a, b| a.identity == b.identity);

        super::core::protect_core_package(&mut packages, &self.local.package_store()?)?;

        Ok(ImportedPackages { packages, personal, third_party })
    }
}
