use crate::app::SafeAppHandle;
use crate::app::chord_package::ChordPackage;

mod git;
pub use git::*;
mod local;
pub use local::*;

pub struct ChordPackageRegistry {
    pub git: GitChordPackageRegistry,
    pub local: LocalPackageRegistry,
}

impl ChordPackageRegistry {
    pub fn new_unloaded(handle: SafeAppHandle) -> anyhow::Result<Self> {
        Ok(Self {
            git: GitChordPackageRegistry::new(handle.clone())?,
            local: LocalPackageRegistry::new(handle),
        })
    }

    pub fn load_all_chord_packages(&self) -> anyhow::Result<Vec<ChordPackage>> {
        let mut packages = vec![ChordPackage::load_bundled()?];
        packages.extend(self.git.load_all_packages()?);
        packages.extend(self.local.load_all_packages()?);
        Ok(packages)
    }
}
