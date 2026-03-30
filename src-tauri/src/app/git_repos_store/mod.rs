use crate::git::{GitHubRepoRef, clone_repo, clone_repo_at_revision, update_or_clone_repo_at_revision};
use crate::observables::{GitRepo, GitReposObservable, GitReposState, Observable};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_store::{Store, StoreExt};
use crate::app::state::StateSingleton;

pub struct GitReposStore {
    observable: GitReposObservable,
    handle: AppHandle,
}

impl StateSingleton for GitReposStore {
    fn new(handle: AppHandle) -> Self {
        Self { handle, observable: GitReposObservable::placeholder() }
    }
}

impl GitReposStore {
    pub fn store(&self) -> Result<Arc<Store<Wry>>>{
        Ok(self.handle.store("repos.json")?)
    }

    pub fn init(&self, observable: GitReposObservable) -> Result<()> {
        self.observable.init(observable);

        let mut repos = load_repos(self.store()?.as_ref())?;
        let repos_root = self.github_repos_dir()?;

        let mut changed = false;
        for repo in repos.values_mut() {
            let repo_ref = GitHubRepoRef {
                owner: repo.owner.clone(),
                name: repo.name.clone(),
            };
            let expected_path = repo_ref.local_path(&repos_root, repo.pinned_rev.as_deref());
            if repo.local_path != expected_path {
                if repo.local_path.exists() && !expected_path.exists() {
                    log::info!(
                        "Moving repo {} from {} to {}",
                        repo.slug,
                        repo.local_path.display(),
                        expected_path.display()
                    );
                    if let Some(parent) = expected_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Err(e) = fs::rename(&repo.local_path, &expected_path) {
                        log::error!("Failed to move repo {}: {}", repo.slug, e);
                    }
                }
                repo.local_path = expected_path;
                changed = true;
            }
        }

        if changed {
            rewrite_repos(self.store()?.as_ref(), &repos)?;
        }

        self.observable.set_state(GitReposState { repos })?;
        Ok(())
    }

    fn save(&self) -> Result<()> {
        self.store()?.save()?;
        Ok(())
    }

    fn add(&self, repo: GitRepo) -> Result<()> {
        let mut state = self.observable.get_state()?.repos.clone();
        state.insert(repo.slug.clone(), repo);
        self.replace_all(state)
    }

    fn replace_all(&self, repos: HashMap<String, GitRepo>) -> Result<()> {
        rewrite_repos(self.store()?.as_ref(), &repos)?;
        self.observable.set_state(GitReposState { repos })?;
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

        self.replace_all(repos)?;
        let repo_path = PathBuf::from(&removed_repo.local_path);
        remove_repo_cache(&repo_path)?;
        Ok(())
    }

    pub fn github_repos_dir(&self) -> Result<PathBuf> {
        let dir = self.handle.path().app_cache_dir()?;
        Ok(dir.join("repos/github.com"))
    }

    pub fn add_repo(&self, repo_ref: GitHubRepoRef) -> Result<()> {
        let repos_root = self.github_repos_dir()?;
        let repo_path = repo_ref.local_path(&repos_root, None);
        let repo = if repo_path.join(".git").exists() {
            repo_ref.into_repo(&repos_root)
        } else {
            clone_repo(&repo_ref, &repo_path)?;
            repo_ref.into_repo(&repos_root)
        };
        self.add(repo)?;
        Ok(())
    }

    pub fn sync_repo(&self, repo_ref: GitHubRepoRef) -> Result<()> {
        let repos_root = self.github_repos_dir()?;
        let state = self.observable.get_state()?;
        let current_repo = state.repos.get(&repo_ref.slug());
        let pinned_rev = current_repo.and_then(|r| r.pinned_rev.as_deref());

        let repo_path = repo_ref.local_path(&repos_root, pinned_rev);

        if !repo_path.join(".git").exists() {
            anyhow::bail!("Repository {} has not been added yet", repo_ref.slug());
        }

        crate::git::refresh_repo(&repo_ref, &repo_path)?;

        let repo = match pinned_rev {
            Some(rev) => repo_ref.into_pinned_repo(&repos_root, rev),
            None => repo_ref.into_repo(&repos_root),
        };

        let key = repo.slug.clone();
        let value = serde_json::to_value(repo.clone())?;
        self.store()?.set(key.clone(), value);
        self.save()?;

        let mut repos = state.repos.clone();
        repos.insert(key, repo);
        self.observable.set_state(GitReposState { repos })?;

        Ok(())
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
            let repo_path = spec.repo_ref.local_path(&repos_root, Some(&spec.rev));
            // Use the new function to handle both cloning if not exists, and fetching/updating if exists.
            update_or_clone_repo_at_revision(&spec.repo_ref, &repo_path, &spec.rev)?;
            let repo = spec.repo_ref.into_pinned_repo(&repos_root, spec.rev);
            next_repos.insert(repo.slug.clone(), repo);
        }

        self.replace_all(next_repos.clone())?;

        for repo in previous_repos.values() {
            if !desired_slugs.contains(&repo.slug) {
                remove_repo_cache(&PathBuf::from(&repo.local_path))?;
            }
        }

        Ok(next_repos.into_values().collect())
    }

    pub fn ensure_pinned_repos(&self, repos: Vec<PinnedGitRepoSpec>) -> Result<()> {
        let repos_root = self.github_repos_dir()?;
        let state = self.observable.get_state()?;
        let mut current_repos = state.repos.clone();

        for spec in repos {
            let repo_path = spec.repo_ref.local_path(&repos_root, Some(&spec.rev));
            update_or_clone_repo_at_revision(&spec.repo_ref, &repo_path, &spec.rev)?;
            let repo = spec.repo_ref.into_pinned_repo(&repos_root, spec.rev);
            current_repos.insert(repo.slug.clone(), repo);
        }

        self.replace_all(current_repos)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PinnedGitRepoSpec {
    pub repo_ref: GitHubRepoRef,
    pub rev: String,
}

fn load_repos(store: &Store<Wry>) -> Result<HashMap<String, GitRepo>> {
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

fn rewrite_repos(store: &Store<Wry>, repos: &HashMap<String, GitRepo>) -> Result<()> {
    store.clear();
    for (slug, repo) in repos {
        let value = serde_json::to_value(repo)
            .with_context(|| format!("Failed to serialize repo {slug}"))?;
        store.set(slug.clone(), value);
    }
    store.save()?;
    Ok(())
}

fn remove_repo_cache(repo_path: &PathBuf) -> Result<()> {
    if repo_path.exists() {
        fs::remove_dir_all(repo_path).with_context(|| {
            format!(
                "Failed to remove local repo cache at {}",
                repo_path.display()
            )
        })?;

        if let Some(parent) = repo_path.parent() {
            if parent.read_dir()?.next().is_none() {
                let _ = fs::remove_dir(parent);
            }
        }
    }

    Ok(())
}
