use crate::app::SafeAppHandle;
use crate::git::{GitHubRepoRef, clone_repo, clone_repo_at_revision};
use crate::observables::{GitRepo, GitReposObservable, GitReposState, Observable};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Wry;
use tauri_plugin_store::Store;

#[derive(Clone)]
pub struct GitReposStore {
    pub store: Arc<Store<Wry>>,
    observable: Arc<GitReposObservable>,

    handle: SafeAppHandle,
}

impl GitReposStore {
    pub fn new(handle: SafeAppHandle, observable: Arc<GitReposObservable>) -> Result<Self> {
        let store = handle.store("repos.json")?;
        let repos = load_repos(store.as_ref())?;
        log::debug!("repos: {:?}", repos);
        observable.set_state(GitReposState { repos })?;
        Ok(Self {
            handle,
            store,
            observable,
        })
    }

    fn save(&self) -> Result<()> {
        self.store.save()?;
        Ok(())
    }

    fn add(&self, repo: GitRepo) -> Result<()> {
        let mut state = self.observable.get_state()?.repos.clone();
        state.insert(repo.slug.clone(), repo);
        self.replace_all(state)
    }

    fn replace_all(&self, repos: HashMap<String, GitRepo>) -> Result<()> {
        rewrite_repos(self.store.as_ref(), &repos)?;
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
        let repo_path = repo_ref.local_path(&repos_root);
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
        let repo_path = repo_ref.local_path(&repos_root);

        if !repo_path.join(".git").exists() {
            anyhow::bail!("Repository {} has not been added yet", repo_ref.slug());
        }

        crate::git::refresh_repo(&repo_ref, &repo_path)?;

        let repo = repo_ref.into_repo(&repos_root);
        let key = repo.slug.clone();
        let value = serde_json::to_value(repo.clone())?;
        self.store.set(key.clone(), value);
        self.save()?;

        let state = self.observable.get_state()?;
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
            let repo_path = spec.repo_ref.local_path(&repos_root);
            clone_repo_at_revision(&spec.repo_ref, &repo_path, &spec.rev)?;
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
