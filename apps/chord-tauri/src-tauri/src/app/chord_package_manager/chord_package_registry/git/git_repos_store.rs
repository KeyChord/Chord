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
            let repo_ref = GitHubRepoRef {
                owner: repo.owner.clone(),
                name: repo.name.clone(),
            };
            let Ok(resolved_rev) = repo_head_sha(&repo.local_abspath) else {
                log::warn!(
                    "Unable to resolve cached HEAD for {}; leaving its stored path unchanged",
                    repo.slug
                );
                continue;
            };
            let expected_path = repo_ref.local_abspath(&repos_root, &resolved_rev);
            if repo.local_abspath != expected_path {
                if repo.local_abspath.exists() && !expected_path.exists() {
                    log::info!(
                        "Moving repo {} from {} to {}",
                        repo.slug,
                        repo.local_abspath.display(),
                        expected_path.display()
                    );
                    if let Some(parent) = expected_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Err(e) = fs::rename(&repo.local_abspath, &expected_path) {
                        log::error!("Failed to move repo {}: {}", repo.slug, e);
                    }
                }
                repo.local_abspath = expected_path;
                changed = true;
            }
            repo.head_short_sha = Some(resolved_rev.chars().take(7).collect());
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
        Ok(self.app_cache_dir()?.join("repos/github.com"))
    }

    pub fn add_repo(&self, repo_ref: GitHubRepoRef) -> Result<GitRepo> {
        if let Some(repo) = self.observable.get_state()?.repos.get(&repo_ref.slug()) {
            if repo.linked_local_path.is_some() || repo.is_monorepo {
                return Ok(repo.clone());
            }
        }
        let repos_root = self.github_repos_dir()?;
        let repo = materialize_repo_head(&repo_ref, &repos_root)?;
        self.upsert(repo.clone())?;
        Ok(repo)
    }

    pub fn add_monorepo(&self, repo_ref: GitHubRepoRef) -> Result<GitRepo> {
        if let Some(existing) = self.observable.get_state()?.repos.get(&repo_ref.slug()) {
            anyhow::ensure!(
                existing.is_monorepo,
                "This repository is already added as a single package. Remove it before adding it as a monorepo."
            );
            return Ok(existing.clone());
        }
        let mut repo = materialize_repo_head(&repo_ref, &self.github_repos_dir()?)?;
        super::super::validate_monorepo_path(&repo.local_abspath.to_string_lossy())?;
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
        if repo.is_monorepo {
            super::super::validate_monorepo_path(&repo.local_abspath.to_string_lossy())?;
        }
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
        for spec in repos {
            let repo_path = spec.repo_ref.local_abspath(&repos_root, &spec.rev);
            materialize_repo_at_revision(&spec.repo_ref, &repo_path, &spec.rev)?;
            let repo = spec.repo_ref.into_pinned_repo(&repos_root, spec.rev);
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

        for spec in repos {
            let repo_path = spec.repo_ref.local_abspath(&repos_root, &spec.rev);
            materialize_repo_at_revision(&spec.repo_ref, &repo_path, &spec.rev)?;
            let mut repo = spec.repo_ref.into_pinned_repo(&repos_root, spec.rev);
            repo.linked_local_path = current_repos
                .get(&repo.slug)
                .and_then(|previous| previous.linked_local_path.clone());
            repo.is_monorepo = current_repos
                .get(&repo.slug)
                .is_some_and(|previous| previous.is_monorepo);
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
                    super::super::validate_monorepo_path(&path)
                } else {
                    validate_local_link(&repo.local_abspath, &path)
                }
            })
            .transpose()?;
        self.upsert(repo.clone())?;
        Ok(repo)
    }
}

fn validate_local_link(cached_path: &std::path::Path, path: &str) -> Result<PathBuf> {
    use super::super::LocalPackageRegistry;

    anyhow::ensure!(!path.trim().is_empty(), "Folder path cannot be empty");
    let path = fs::canonicalize(path.trim()).context("Failed to access linked folder")?;
    anyhow::ensure!(path.is_dir(), "{} is not a folder", path.display());
    let original = LocalPackageRegistry::import_from_local_folder(cached_path)?;
    let linked = LocalPackageRegistry::import_from_local_folder(&path)?;
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

    #[test]
    fn local_link_validates_identity_and_reads_edits_from_checkout() {
        let fixture = Fixture::new();
        let cached = fixture.0.join("cached");
        let checkout = fixture.0.join("my checkout");
        let path = validate_local_link(&cached, &format!(" {} ", checkout.display())).unwrap();
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
            validate_local_link(&cached, checkout.to_str().unwrap())
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
            assert!(validate_local_link(&cached, &path).is_err());
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
