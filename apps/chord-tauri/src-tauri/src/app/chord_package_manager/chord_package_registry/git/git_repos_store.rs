use super::super::PackageStore;
use crate::app::state::AppSingleton;
use crate::git::{
    GitHubRepoRef, materialize_repo_at_revision, materialize_repo_head, repo_head_sha,
};
use crate::state::{GitRepo, GitReposObservable, GitReposState, Observable};
use anyhow::{Context, Result};
use nject::injectable;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_store::{Store, StoreExt};

/// TODO: This should not be a generic "git repos store", but instead should be tailored to the
/// specific use cases of the Git package registry
#[injectable]
pub struct GitReposStore {
    observable: GitReposObservable,
    handle: AppHandle,
}

impl GitReposStore {
    pub(in super::super) fn init(&self) -> Result<()> {
        let mut repos = load_repos(self.store()?.as_ref())?;
        let repos_root = self.github_repos_dir()?;

        let mut changed = false;
        for repo in repos.values_mut() {
            let Ok(resolved_rev) = stored_checkout_revision(repo, &repos_root) else {
                log::warn!(
                    "Unable to resolve cached HEAD for {}; leaving its stored path unchanged",
                    repo.slug
                );
                continue;
            };
            match migrate_checkout(repo, &repos_root, &resolved_rev) {
                Ok(migrated) => changed |= migrated,
                Err(error) => {
                    log::error!(
                        "Cannot migrate {} into ~/.chord/packages: {error:#}",
                        repo.slug
                    );
                    continue;
                }
            }
            let was_monorepo = repo.is_monorepo;
            if let Err(error) = detect_monorepo(&PackageStore::for_app(&self.handle)?, repo) {
                log::warn!("Unable to detect packages in {}: {error}", repo.slug);
            }
            changed |= repo.is_monorepo != was_monorepo;
        }

        if changed {
            rewrite_repos(self.store()?.as_ref(), &repos)?;
        }

        self.observable.set_state(|_| GitReposState { repos })?;
        Ok(())
    }

    pub fn app_cache_dir(&self) -> Result<PathBuf> {
        Ok(self.handle.path().app_cache_dir()?)
    }

    pub fn store(&self) -> Result<Arc<Store<Wry>>> {
        Ok(self.handle.store("repos.json")?)
    }

    pub fn has_persisted_state(&self) -> Result<bool> {
        let path = tauri_plugin_store::resolve_store_path(&self.handle, "repos.json")?;
        Ok(path.exists())
    }

    fn save(&self) -> Result<()> {
        self.store()?.save()?;
        Ok(())
    }

    fn upsert(&self, repo: GitRepo) -> Result<()> {
        let key = repo.slug.clone();
        self.store()?.set(key.clone(), serde_json::to_value(&repo)?);
        self.save()?;
        self.observable.set_state(|prev| {
            let mut next = prev;
            next.repos.insert(key, repo);
            next
        })?;
        Ok(())
    }

    fn replace_all(&self, repos: HashMap<String, GitRepo>) -> Result<()> {
        rewrite_repos(self.store()?.as_ref(), &repos)?;
        self.observable.set_state(|_| GitReposState { repos })?;
        Ok(())
    }

    pub fn remove_repo(&self, slug: &str) -> Result<()> {
        let id = slug.trim().to_string();
        anyhow::ensure!(!id.is_empty(), "Repository cannot be empty");
        ensure_repo_removable(&id)?;

        let state = self.observable.get_state()?;
        let mut repos = state.repos.clone();
        let removed_repo = repos
            .remove(&id)
            .with_context(|| format!("Repository {id} has not been added yet"))?;

        self.store()?.delete(&id);
        self.save()?;
        self.observable.set_state(|_| GitReposState { repos })?;
        log::debug!(
            "Removed active repository {}; retained immutable cache entry at {}",
            id,
            removed_repo.local_abspath.display()
        );
        Ok(())
    }

    pub fn github_repos_dir(&self) -> Result<PathBuf> {
        Ok(PackageStore::for_app(&self.handle)?.github_dir())
    }

    pub fn add_repo(&self, repo_ref: GitHubRepoRef) -> Result<GitRepo> {
        let repo_ref = GitHubRepoRef::parse(&repo_ref.slug())?;
        ensure_official_install(&repo_ref)?;
        if let Some(repo) = self
            .observable
            .get_state()?
            .repos
            .values()
            .find(|repo| repo.slug.eq_ignore_ascii_case(&repo_ref.slug()))
        {
            // Adding an already installed package is idempotent, including pinned versions.
            return Ok(repo.clone());
        }
        let repos_root = self.github_repos_dir()?;
        let mut repo = materialize_repo_head(&repo_ref, &repos_root)?;
        detect_monorepo(&PackageStore::for_app(&self.handle)?, &mut repo)?;
        self.upsert(repo.clone())?;
        Ok(repo)
    }

    pub fn add_monorepo(&self, repo_ref: GitHubRepoRef) -> Result<GitRepo> {
        let repo_ref = GitHubRepoRef::parse(&repo_ref.slug())?;
        ensure_official_install(&repo_ref)?;
        if let Some(existing) = self
            .observable
            .get_state()?
            .repos
            .values()
            .find(|repo| repo.slug.eq_ignore_ascii_case(&repo_ref.slug()))
        {
            anyhow::ensure!(
                existing.is_monorepo,
                "This repository is already added as a single package. Remove it before adding it as a monorepo."
            );
            return Ok(existing.clone());
        }
        let mut repo = materialize_repo_head(&repo_ref, &self.github_repos_dir()?)?;
        super::super::validate_monorepo_path(
            &PackageStore::for_app(&self.handle)?,
            &repo.local_abspath.to_string_lossy(),
        )?;
        repo.is_monorepo = true;
        self.upsert(repo.clone())?;
        Ok(repo)
    }

    pub fn sync_repo(&self, repo_ref: GitHubRepoRef) -> Result<GitRepo> {
        let repos_root = self.github_repos_dir()?;
        let state = self.observable.get_state()?;
        let current_repo = state
            .repos
            .get(&repo_ref.slug())
            .with_context(|| format!("Repository {} has not been added yet", repo_ref.slug()))?;
        anyhow::ensure!(
            current_repo.linked_local_path.is_none(),
            "Unlink repository {} before syncing",
            repo_ref.slug()
        );
        anyhow::ensure!(
            current_repo.pinned_rev.is_none(),
            "Pinned repository {} cannot be synced to HEAD",
            repo_ref.slug()
        );

        let mut repo = materialize_repo_head(&repo_ref, &repos_root)?;
        repo.is_monorepo = current_repo.is_monorepo;
        detect_monorepo(&PackageStore::for_app(&self.handle)?, &mut repo)?;
        self.upsert(repo.clone())?;
        Ok(repo)
    }

    pub fn replace_with_pinned_repos(&self, repos: Vec<PinnedGitRepoSpec>) -> Result<Vec<GitRepo>> {
        let repos_root = self.github_repos_dir()?;
        let previous_repos = self.observable.get_state()?.repos.clone();
        let desired_slugs = repos
            .iter()
            .map(|repo| repo.repo_ref.slug())
            .collect::<HashSet<_>>();

        let mut next_repos = HashMap::with_capacity(repos.len());
        for mut spec in repos {
            spec.repo_ref = GitHubRepoRef::parse(&spec.repo_ref.slug())?;
            spec.rev = crate::git::normalize_revision(&spec.rev)?;
            let repo_path = spec.repo_ref.local_abspath(&repos_root, &spec.rev);
            materialize_repo_at_revision(&spec.repo_ref, &repo_path, &spec.rev)?;
            let mut repo = spec.repo_ref.into_pinned_repo(&repos_root, spec.rev);
            detect_monorepo(&PackageStore::for_app(&self.handle)?, &mut repo)?;
            next_repos.insert(repo.slug.clone(), repo);
        }

        self.replace_all(next_repos.clone())?;

        for repo in previous_repos
            .values()
            .filter(|repo| !desired_slugs.contains(&repo.slug))
        {
            log::debug!(
                "Deactivated repository {}; retained immutable cache entry at {}",
                repo.slug,
                repo.local_abspath.display()
            );
        }

        Ok(next_repos.into_values().collect())
    }

    pub fn ensure_pinned_repos(&self, repos: Vec<PinnedGitRepoSpec>) -> Result<()> {
        let repos_root = self.github_repos_dir()?;
        let state = self.observable.get_state()?;
        let mut current_repos = state.repos.clone();

        for mut spec in repos {
            spec.repo_ref = GitHubRepoRef::parse(&spec.repo_ref.slug())?;
            spec.rev = crate::git::normalize_revision(&spec.rev)?;
            let repo_path = spec.repo_ref.local_abspath(&repos_root, &spec.rev);
            materialize_repo_at_revision(&spec.repo_ref, &repo_path, &spec.rev)?;
            let mut repo = spec.repo_ref.into_pinned_repo(&repos_root, spec.rev);
            repo.linked_local_path = current_repos
                .get(&repo.slug)
                .and_then(|previous| previous.linked_local_path.clone());
            repo.is_monorepo = current_repos
                .get(&repo.slug)
                .is_some_and(|previous| previous.is_monorepo);
            detect_monorepo(&PackageStore::for_app(&self.handle)?, &mut repo)?;
            current_repos.insert(repo.slug.clone(), repo);
        }

        for repo in current_repos.values() {
            self.store()?
                .set(repo.slug.clone(), serde_json::to_value(repo)?);
        }
        self.save()?;
        self.observable.set_state(|_| GitReposState {
            repos: current_repos,
        })?;
        Ok(())
    }

    pub fn set_local_link(&self, slug: &str, path: Option<String>) -> Result<GitRepo> {
        let mut repo = self
            .observable
            .get_state()?
            .repos
            .get(slug)
            .with_context(|| format!("Repository {slug} has not been added yet"))?
            .clone();
        repo.linked_local_path = path
            .map(|path| {
                if repo.is_monorepo {
                    super::super::validate_monorepo_path(
                        &PackageStore::for_app(&self.handle)?,
                        &path,
                    )
                } else {
                    validate_local_link(
                        &PackageStore::for_app(&self.handle)?,
                        &repo.local_abspath,
                        &path,
                    )
                }
            })
            .transpose()?;
        self.upsert(repo.clone())?;
        Ok(repo)
    }
}

fn ensure_official_install(repo: &GitHubRepoRef) -> Result<()> {
    anyhow::ensure!(super::git::is_official_repo(&repo.slug()),
        "Third-party packages must be downloaded and extracted manually into ~/.chord/packages/<name>. Quit and restart Chord, then approve the package in the Chords tab. Only install packages from people you trust or after auditing their code.");
    Ok(())
}

#[cfg(test)]
mod installation_trust_tests {
    use super::*;

    #[test]
    fn third_party_repositories_require_manual_installation_before_any_download() {
        for source in ["someone/chords-test", "https://github.com/not-keychord/chords-menu"] {
            let error = ensure_official_install(&GitHubRepoRef::parse(source).unwrap()).unwrap_err().to_string();
            assert!(error.contains("~/.chord/packages/<name>"));
            assert!(error.contains("Quit and restart Chord"));
            assert!(error.contains("approve the package in the Chords tab"));
        }
        ensure_official_install(&GitHubRepoRef::parse("KeyChord/chords-menu").unwrap()).unwrap();
    }
}

pub(crate) fn stored_checkout_revision(repo: &GitRepo, root: &std::path::Path) -> Result<String> {
    if let Ok(revision) = repo_head_sha(&repo.local_abspath) {
        return Ok(revision);
    }
    // Recover if the app stopped after moving a checkout but before saving repos.json.
    let revision = repo
        .pinned_rev
        .as_deref()
        .or_else(|| {
            repo.local_abspath
                .file_name()
                .and_then(|name| name.to_str())
        })
        .context("Repository has no recoverable revision")?;
    let revision = crate::git::normalize_revision(revision)?;
    let reference = GitHubRepoRef::parse(&format!("{}/{}", repo.owner, repo.name))?;
    crate::git::verify_repo_at_revision(&reference.local_abspath(root, &revision), &revision)?;
    Ok(revision)
}

pub(crate) fn migrate_checkout(
    repo: &mut GitRepo,
    root: &std::path::Path,
    revision: &str,
) -> Result<bool> {
    let revision = crate::git::normalize_revision(revision)?;
    let reference = GitHubRepoRef::parse(&format!("{}/{}", repo.owner, repo.name))?;
    let expected = reference.local_abspath(root, &revision);
    let changed = repo.local_abspath != expected;
    if changed {
        if expected.exists() {
            crate::git::verify_repo_at_revision(&expected, &revision)?;
        } else {
            super::super::package_store::migrate_directory(&repo.local_abspath, &expected)?;
        }
        repo.local_abspath = expected;
    }
    let short = Some(revision.chars().take(7).collect());
    let changed = changed || repo.head_short_sha != short;
    repo.head_short_sha = short;
    Ok(changed)
}

fn ensure_repo_removable(slug: &str) -> Result<()> {
    anyhow::ensure!(
        !slug.eq_ignore_ascii_case(crate::models::CORE_CHORD_REPO),
        "Chord's core bindings are required and cannot be removed"
    );
    Ok(())
}

#[cfg(test)]
mod core_protection_tests {
    use super::*;

    #[test]
    fn core_repo_cannot_be_removed_but_other_pinned_repos_can() {
        for slug in [
            "KeyChord/chords-chord",
            "keychord/chords-chord",
            "KEYCHORD/CHORDS-CHORD",
        ] {
            assert!(ensure_repo_removable(slug).is_err());
        }
        ensure_repo_removable("KeyChord/chords-os").unwrap();
    }
}

fn detect_monorepo(store: &PackageStore, repo: &mut GitRepo) -> Result<()> {
    // A standalone package owns its root chords/ tree, including any vendored packages.
    if repo.is_monorepo
        || (!repo.local_abspath.join("chords").is_dir()
            && !super::super::discover_monorepo_packages(&repo.local_abspath)?.is_empty())
    {
        super::super::validate_monorepo_path(store, &repo.local_abspath.to_string_lossy())?;
        repo.is_monorepo = true;
    }
    Ok(())
}

fn validate_local_link(
    store: &PackageStore,
    cached_path: &std::path::Path,
    path: &str,
) -> Result<PathBuf> {
    anyhow::ensure!(!path.trim().is_empty(), "Folder path cannot be empty");
    let path = fs::canonicalize(path.trim()).context("Failed to access linked folder")?;
    anyhow::ensure!(path.is_dir(), "{} is not a folder", path.display());
    let original = store.import_package(cached_path)?;
    let linked = store.import_package(&path)?;
    anyhow::ensure!(
        original.package_name() == linked.package_name(),
        "Expected package {}, but this folder contains {}. Use a checkout with the same package.json name.",
        original.package_name(),
        linked.package_name()
    );
    Ok(path)
}

#[derive(Debug, Clone)]
pub struct PinnedGitRepoSpec {
    pub repo_ref: GitHubRepoRef,
    pub rev: String,
}

pub fn load_repos(store: &Store<Wry>) -> Result<HashMap<String, GitRepo>> {
    let entries = store.entries();
    let mut repos = HashMap::new();
    let mut should_rewrite = false;

    for (key, value) in entries {
        match serde_json::from_value::<GitRepo>(value) {
            Ok(repo) => {
                if key != repo.slug {
                    log::warn!(
                        "Normalizing git repo store key from {} to {}",
                        key,
                        repo.slug
                    );
                    should_rewrite = true;
                }

                repos.insert(repo.slug.clone(), repo);
            }
            Err(error) => {
                log::warn!("Skipping invalid git repo store entry {key}: {error}");
                should_rewrite = true;
            }
        }
    }

    if should_rewrite {
        rewrite_repos(store, &repos)?;
    }

    Ok(repos)
}

pub fn rewrite_repos(store: &Store<Wry>, repos: &HashMap<String, GitRepo>) -> Result<()> {
    let desired_slugs = repos.keys().cloned().collect::<HashSet<_>>();
    for (slug, _) in store.entries() {
        if !desired_slugs.contains(&slug) {
            store.delete(&slug);
        }
    }
    for (slug, repo) in repos {
        let value = serde_json::to_value(repo)
            .with_context(|| format!("Failed to serialize repo {slug}"))?;
        store.set(slug.clone(), value);
    }
    store.save()?;
    Ok(())
}

#[cfg(test)]
mod local_link_tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn store(&self) -> PackageStore {
            PackageStore::new(self.0.join(".store"))
        }
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("chord-local-link-{}", uuid::Uuid::new_v4()));
            for folder in ["cached", "my checkout"] {
                fs::create_dir_all(root.join(folder).join("chords")).unwrap();
                fs::write(
                    root.join(folder).join("package.json"),
                    r#"{"name":"@test/chords"}"#,
                )
                .unwrap();
            }
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_repo(root: PathBuf) -> GitRepo {
        serde_json::from_value(serde_json::json!({
            "owner": "test", "name": "repo", "slug": "test/repo",
            "url": "https://github.com/test/repo", "localAbspath": root,
            "headShortSha": null
        }))
        .unwrap()
    }

    #[test]
    fn auto_detection_finds_root_and_nested_packages() {
        let fixture = Fixture::new();
        let root = fixture.0.join("monorepo");
        fs::create_dir_all(&root).unwrap();
        let mut repo = test_repo(root.clone());
        for folder in [
            "chords-one",
            "group/deeper/chords-two",
            "packages/chords-three",
        ] {
            let path = root.join(folder);
            fs::create_dir_all(path.join("chords")).unwrap();
            detect_monorepo(&fixture.store(), &mut repo).unwrap();
            assert!(repo.is_monorepo);
            fs::remove_dir_all(path).unwrap();
            repo.is_monorepo = false;
        }
        fs::create_dir_all(root.join("utility")).unwrap();
        fs::create_dir_all(root.join("node_modules/chords-dependency")).unwrap();
        detect_monorepo(&fixture.store(), &mut repo).unwrap();
        assert!(!repo.is_monorepo);
    }

    #[test]
    fn auto_detection_keeps_standalone_packages_with_vendored_chords() {
        let fixture = Fixture::new();
        let root = fixture.0.join("cached");
        fs::create_dir_all(root.join("chords/@test/chords-vendored")).unwrap();
        let mut repo = test_repo(root);
        detect_monorepo(&fixture.store(), &mut repo).unwrap();
        assert!(!repo.is_monorepo);
    }

    #[test]
    fn auto_detection_rejects_duplicate_package_names_before_classifying() {
        let fixture = Fixture::new();
        let root = fixture.0.join("monorepo");
        for folder in ["chords-one", "group/chords-two"] {
            let path = root.join(folder);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("package.json"), r#"{"name":"@test/duplicate"}"#).unwrap();
        }
        let mut repo = test_repo(root);
        assert!(detect_monorepo(&fixture.store(), &mut repo).is_err());
        assert!(!repo.is_monorepo);
    }

    #[test]
    fn local_link_validates_identity_and_reads_edits_from_checkout() {
        let fixture = Fixture::new();
        let cached = fixture.0.join("cached");
        let checkout = fixture.0.join("my checkout");
        let path = validate_local_link(
            &fixture.store(),
            &cached,
            &format!(" {} ", checkout.display()),
        )
        .unwrap();
        assert_eq!(path, fs::canonicalize(&checkout).unwrap());
        for contents in ["first edit", "second edit"] {
            fs::write(checkout.join("chords/macos.toml"), contents).unwrap();
            let package =
                super::super::super::LocalPackageRegistry::import_from_local_folder(&path).unwrap();
            assert_eq!(package.root, path);
            assert_eq!(
                package.chords_files_contents[std::path::Path::new("chords/macos.toml")],
                contents
            );
        }
        fs::write(checkout.join("package.json"), r#"{"name":"@test/wrong"}"#).unwrap();
        assert!(
            validate_local_link(&fixture.store(), &cached, checkout.to_str().unwrap())
                .unwrap_err()
                .to_string()
                .contains("Expected package")
        );
    }

    #[test]
    fn local_link_rejects_empty_missing_and_file_paths() {
        let fixture = Fixture::new();
        let cached = fixture.0.join("cached");
        for path in [
            String::new(),
            "  ".into(),
            fixture.0.join("missing").display().to_string(),
            cached.join("package.json").display().to_string(),
        ] {
            assert!(validate_local_link(&fixture.store(), &cached, &path).is_err());
        }
    }

    #[test]
    fn local_link_is_optional_in_old_stores_and_persists_in_new_stores() {
        let legacy = serde_json::json!({"owner":"test", "name":"chords", "slug":"test/chords", "url":"https://github.com/test/chords", "localAbspath":"/cache/chords", "headShortSha":null, "pinnedRev":"abc"});
        let mut repo: GitRepo = serde_json::from_value(legacy).unwrap();
        assert!(repo.linked_local_path.is_none());
        repo.linked_local_path = Some(PathBuf::from("/my checkout"));
        let restored: GitRepo =
            serde_json::from_value(serde_json::to_value(&repo).unwrap()).unwrap();
        assert_eq!(restored.linked_local_path, repo.linked_local_path);
        assert_eq!(restored.local_abspath, repo.local_abspath);
        assert_eq!(restored.pinned_rev, repo.pinned_rev);
    }
}
