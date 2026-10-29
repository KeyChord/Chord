use super::{GitReposStore, load_repos};
use crate::app::AppHandleExt;
use crate::app::chord_package_manager::chord_package_registry::PackageStore;
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
            &PackageStore::for_app(&self.handle)?,
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
    store: &PackageStore,
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
        if !linked && !store.contains(path) {
            log::warn!(
                "Skipping {}: its checkout has not been migrated into ~/.chord/packages",
                repo.slug
            );
            continue;
        }
        let mut imported = if repo.is_monorepo {
            super::super::import_selected_monorepo_packages(
                store,
                path,
                selections
                    .get(&repo.slug)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )?
        } else if linked {
            anyhow::ensure!(
                path.is_dir(),
                "Linked folder for {} is unavailable: {}",
                repo.slug,
                path.display()
            );
            let package = store.import_package(path)?;
            HashMap::from([(package.package_name(), package)])
        } else if let Ok(package) = store
            .import_package(path)
            .inspect_err(|e| log::warn!("skipping repo {} because of import error: {e}", repo.slug))
        {
            HashMap::from([(package.package_name(), package)])
        } else { HashMap::new() };
        // Local links still require approval, even when attached to an official repo.
        if !linked && is_official_repo(&repo.slug) {
            for package in imported.values_mut() {
                if package.package_name().starts_with("@keychord/") {
                    package.root = store.mount_official(&package.root)?;
                }
            }
        }
        packages.extend(imported);
    }
    Ok(packages)
}

pub(super) fn is_official_repo(slug: &str) -> bool {
    // The official GitHub organization owns the mirrors, including internal packages
    // that are not listed in Browse. A third party's manifest cannot claim this origin.
    GitHubRepoRef::parse(slug).is_ok_and(|repo| {
        let name = repo.name.to_ascii_lowercase();
        repo.owner.eq_ignore_ascii_case("keychord") && (name == "chords" || name.starts_with("chords-"))
    })
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
    fn official_downloads_use_the_trusted_namespace_but_local_links_still_need_approval() {
        let fixture = Fixture(std::env::temp_dir().join(format!("chord-official-source-{}", uuid::Uuid::new_v4())));
        let store = PackageStore::new(fixture.0.join("packages"));
        let remote = store.root.join(".sources/github.com/keychord/chords-menu/revision");
        fs::create_dir_all(&remote).unwrap();
        fs::write(remote.join("package.json"), r#"{"name":"@keychord/chords-menu"}"#).unwrap();
        let official = repo("keychord/chords-menu", &remote, None, false);
        let packages = import_repo_packages(&store, vec![&official], false, &HashMap::new()).unwrap();
        let package = &packages["@keychord/chords-menu"];
        assert!(store.is_official_root(&package.root));
        assert_eq!(fs::canonicalize(&package.root).unwrap(), fs::canonicalize(&remote).unwrap());
        let impersonator = repo("someone/chords-menu", &remote, None, false);
        let packages = import_repo_packages(&store, vec![&impersonator], false, &HashMap::new()).unwrap();
        assert!(!store.is_official_root(&packages["@keychord/chords-menu"].root));
        let linked = repo("keychord/chords-menu", &remote, Some(&remote), false);
        let packages = import_repo_packages(&store, vec![&linked], true, &HashMap::new()).unwrap();
        assert!(!store.is_official_root(&packages["@keychord/chords-menu"].root));
        for official in ["KeyChord/chords", "KEYCHORD/CHORDS-CHORD", "keychord/chords-os", "keychord/chords-vscode", "keychord/chords-tray", "keychord/chords-commands"] {
            assert!(is_official_repo(official), "{official}");
        }
        assert!(!is_official_repo("other/chords-chord"));
        assert!(!is_official_repo("keychord/not-an-official-package"));
    }

    #[test]
    fn monorepo_local_links_override_remote_packages_regardless_of_repo_order() {
        let mut fixture = Fixture(
            std::env::temp_dir().join(format!("chord-precedence-{}", uuid::Uuid::new_v4())),
        );
        fs::create_dir_all(&fixture.0).unwrap();
        fixture.0 = fs::canonicalize(&fixture.0).unwrap();
        let store = PackageStore::new(fixture.0.join(".store"));
        let remote = store.root.join("remote");
        let local = fixture.0.join("local");
        let monorepo = store.root.join("monorepo");
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
            import_repo_packages(&store, vec![&local_repo], true, &HashMap::new())
                .unwrap()
                .is_empty()
        );
        let unselected_remote = repo("unselected", &monorepo, None, true);
        assert!(
            import_repo_packages(&store, vec![&unselected_remote], false, &HashMap::new())
                .unwrap()
                .is_empty()
        );
        for repos in [
            vec![&remote_repo, &local_repo],
            vec![&local_repo, &remote_repo],
        ] {
            let mut packages =
                import_repo_packages(&store, repos.clone(), false, &selections).unwrap();
            assert_eq!(
                fs::canonicalize(&packages["@test/shared"].root).unwrap(),
                remote
            );
            packages.extend(import_repo_packages(&store, repos, true, &selections).unwrap());
            assert!(
                packages
                    .values()
                    .all(|package| store.contains(&package.root))
            );
            assert_eq!(
                fs::canonicalize(&packages["@test/shared"].root).unwrap(),
                child
            );
        }
        // An explicitly linked single package also overrides a remote monorepo.
        let remote_monorepo = repo("z-monorepo", &monorepo, None, true);
        let local_single = repo("a-single", &remote, Some(&local), false);
        let repos = vec![&local_single, &remote_monorepo];
        let mut packages = import_repo_packages(&store, repos.clone(), false, &selections).unwrap();
        assert_eq!(
            fs::canonicalize(&packages["@test/shared"].root).unwrap(),
            child
        );
        packages.extend(import_repo_packages(&store, repos, true, &selections).unwrap());
        assert_eq!(
            fs::canonicalize(&packages["@test/shared"].root).unwrap(),
            local
        );
        // Unlinking restores the cached source.
        let unlinked = repo("a-single", &remote, None, false);
        assert!(
            import_repo_packages(&store, vec![&unlinked], true, &selections)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            import_repo_packages(&store, vec![&unlinked], false, &selections).unwrap()["@test/shared"].root,
            remote
        );
    }

    #[test]
    fn an_unmigrated_download_is_never_loaded_from_the_old_cache() {
        let root = std::env::temp_dir().join(format!("chord-old-cache-{}", uuid::Uuid::new_v4()));
        let fixture = Fixture(root);
        let old_cache = fixture.0.join("old-cache");
        fs::create_dir_all(&old_cache).unwrap();
        fs::write(
            old_cache.join("package.json"),
            r#"{"name":"test-old-cache"}"#,
        )
        .unwrap();
        let store = PackageStore::new(fixture.0.join("packages"));
        let remote = repo("test/old-cache", &old_cache, None, false);
        assert!(
            import_repo_packages(&store, vec![&remote], false, &HashMap::new())
                .unwrap()
                .is_empty()
        );
        assert!(!store.root.exists());
    }
}
