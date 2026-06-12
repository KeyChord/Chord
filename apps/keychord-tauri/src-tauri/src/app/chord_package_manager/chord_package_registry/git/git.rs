use super::{GitReposStore, load_repos};
use crate::app::AppHandleExt;
use crate::app::chord_package_manager::chord_package_registry::LocalPackageRegistry;
use crate::app::state::AppSingleton;
use crate::git::GitHubRepoRef;
use crate::models::RawChordPackage;
use crate::state::{GitRepo, GitReposObservable, GitReposState, Observable};
use anyhow::Result;
use nject::injectable;
use std::collections::HashMap;
use std::fs;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

#[injectable]
pub struct GitChordPackageRegistry {
    pub git_repos_store: GitReposStore,
}

impl GitChordPackageRegistry {
    pub(super) fn init(&self) -> Result<()> {
        self.git_repos_store.init()
    }

    pub fn import_all_packages(&self) -> Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

        // TODO: this signature is bad
        for repo in load_repos(self.git_repos_store.store()?.as_ref())?.values() {
            if let Ok(package) =
                LocalPackageRegistry::import_from_local_folder(repo.local_abspath.as_path())
                    .inspect_err(|e| {
                        log::warn!("skipping repo {} because of import error: {e}", repo.slug)
                    })
            {
                packages.insert(package.package_name(), package);
            }
        }

        Ok(packages)
    }
}
