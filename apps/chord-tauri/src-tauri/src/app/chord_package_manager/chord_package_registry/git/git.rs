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
    handle: AppHandle,
}

impl GitChordPackageRegistry {
    pub(in crate::app::chord_package_manager) fn init(&self) -> Result<()> {
        self.git_repos_store.init()
    }

    pub fn import_packages(&self, linked: bool) -> Result<HashMap<String, RawChordPackage>> {
        let repos = load_repos(self.git_repos_store.store()?.as_ref())?;
        import_repo_packages(
            repos.values().collect(),
            linked,
            &self
                .handle
                .app_state()
                .chord_package_manager()
                .registry
                .local
                .monorepo_selections()?,
        )
    }
}

fn import_repo_packages(
    mut repos: Vec<&GitRepo>,
    linked: bool,
    selections: &HashMap<String, Vec<String>>,
) -> Result<HashMap<String, RawChordPackage>> {
    let mut packages = HashMap::new();
    // Stable precedence within a source tier, independent of HashMap iteration order.
    repos.sort_by(|a, b| a.slug.cmp(&b.slug));
    for repo in repos {
        if repo.linked_local_path.is_some() != linked {
            continue;
        }
        let path = repo
            .linked_local_path
            .as_ref()
            .unwrap_or(&repo.local_abspath);
        if repo.is_monorepo {
            packages.extend(super::super::import_selected_monorepo_packages(
                path,
                selections
                    .get(&repo.slug)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )?);
        } else if linked {
            anyhow::ensure!(
                path.is_dir(),
                "Linked folder for {} is unavailable: {}",
                repo.slug,
                path.display()
            );
            let package = LocalPackageRegistry::import_from_local_folder(path)?;
            packages.insert(package.package_name(), package);
        } else if let Ok(package) = LocalPackageRegistry::import_from_local_folder(path)
            .inspect_err(|e| log::warn!("skipping repo {} because of import error: {e}", repo.slug))
        {
            packages.insert(package.package_name(), package);
        }
    }
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn repo(slug: &str, root: &Path, linked: Option<&Path>, monorepo: bool) -> GitRepo {
        serde_json::from_value(serde_json::json!({
            "owner": "test", "name": slug, "slug": slug, "url": "https://github.com/test/example",
            "localAbspath": root, "linkedLocalPath": linked, "isMonorepo": monorepo,
            "headShortSha": null, "pinnedRev": null
        }))
        .unwrap()
    }

    #[test]
    fn monorepo_local_links_override_remote_packages_regardless_of_repo_order() {
        let fixture = Fixture(
            std::env::temp_dir().join(format!("chord-precedence-{}", uuid::Uuid::new_v4())),
        );
        let remote = fixture.0.join("remote");
        let local = fixture.0.join("local");
        let monorepo = fixture.0.join("monorepo");
        let child = monorepo.join("packages/chords-shared");
        for path in [&remote, &local, &child] {
            fs::create_dir_all(path).unwrap();
            fs::write(path.join("package.json"), r#"{"name":"@test/shared"}"#).unwrap();
        }
        let selections = HashMap::from([
            ("a-local".to_owned(), vec!["@test/shared".to_owned()]),
            ("z-monorepo".to_owned(), vec!["@test/shared".to_owned()]),
        ]);
        let remote_repo = repo("z-remote", &remote, None, false);
        let local_repo = repo("a-local", &remote, Some(&monorepo), true);
        assert!(
            import_repo_packages(vec![&local_repo], true, &HashMap::new())
                .unwrap()
                .is_empty()
        );
        let unselected_remote = repo("unselected", &monorepo, None, true);
        assert!(
            import_repo_packages(vec![&unselected_remote], false, &HashMap::new())
                .unwrap()
                .is_empty()
        );
        for repos in [
            vec![&remote_repo, &local_repo],
            vec![&local_repo, &remote_repo],
        ] {
            let mut packages = import_repo_packages(repos.clone(), false, &selections).unwrap();
            assert_eq!(packages["@test/shared"].root, remote);
            packages.extend(import_repo_packages(repos, true, &selections).unwrap());
            assert_eq!(packages["@test/shared"].root, child);
        }
        // An explicitly linked single package also overrides a remote monorepo.
        let remote_monorepo = repo("z-monorepo", &monorepo, None, true);
        let local_single = repo("a-single", &remote, Some(&local), false);
        let repos = vec![&local_single, &remote_monorepo];
        let mut packages = import_repo_packages(repos.clone(), false, &selections).unwrap();
        assert_eq!(packages["@test/shared"].root, child);
        packages.extend(import_repo_packages(repos, true, &selections).unwrap());
        assert_eq!(packages["@test/shared"].root, local);
        // Unlinking restores the cached source.
        let unlinked = repo("a-single", &remote, None, false);
        assert!(
            import_repo_packages(vec![&unlinked], true, &selections)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            import_repo_packages(vec![&unlinked], false, &selections).unwrap()["@test/shared"].root,
            remote
        );
    }
}
