use crate::app::chord_package_manager::ChordPackageManager;
use crate::app::chord_package_manager::chord_package_registry::ChordPackageRegistry;
use crate::app::state::AppSingleton;
use crate::state::{
    ChordPackageManagerObservable, FrontmostObservable, GitReposObservable, Observable,
};
use anyhow::Result;
use nject::provider;
use ordermap::OrderMap;
use parking_lot::RwLock;
use tauri::AppHandle;

#[provider]
pub struct ChordPackageManagerProvider {
    #[provide(ChordPackageManagerObservable, |v| v.provide())]
    pub chord_package_manager_observable: ChordPackageManagerObservable,

    #[provide(GitReposObservable, |v| v.provide())]
    pub git_repos_observable: GitReposObservable,

    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for ChordPackageManager {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
