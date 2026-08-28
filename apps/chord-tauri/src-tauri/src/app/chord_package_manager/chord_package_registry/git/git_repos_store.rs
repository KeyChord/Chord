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
        let repos_root = self.github_repos_dir()?;
        let repo = materialize_repo_head(&repo_ref, &repos_root)?;
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
            current_repo.pinned_rev.is_none(),
            "Pinned repository {} cannot be synced to HEAD",
            repo_ref.slug()
        );

        let repo = materialize_repo_head(&repo_ref, &repos_root)?;
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
            let repo = spec.repo_ref.into_pinned_repo(&repos_root, spec.rev);
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
