use tauri::AppHandle;
use crate::app::chord_package_registry::{ChordPackageRegistry, ConfigPackageRegistry, GitChordPackageRegistry, LocalPackageRegistry};
use crate::app::state::AppSingleton;

impl AppSingleton<()> for ChordPackageRegistry {
    fn new(handle: AppHandle) -> Self {
        Self {
            config: ConfigPackageRegistry::new(),
            git: GitChordPackageRegistry::new(handle.clone()),
            local: LocalPackageRegistry::new(handle.clone()),
        }
    }

    fn init(&self, _: ()) -> anyhow::Result<()> {
        Ok(())
    }
}

