use crate::app::SafeAppHandle;
use crate::chords::ChordPackage;
use crate::registry::{GitPackageRegistry, LocalPackageRegistry};

pub struct ChordPackageRegistry {
    pub git: GitPackageRegistry,
    pub local: LocalPackageRegistry,
}

impl ChordPackageRegistry {
    pub fn new_unloaded(handle: SafeAppHandle) -> anyhow::Result<Self> {
        Ok(Self {
            git: GitPackageRegistry::new(handle.clone())?,
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
