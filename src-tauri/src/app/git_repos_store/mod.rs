use crate::app::SafeAppHandle;
use crate::git::{GitHubRepoRef, clone_repo};
use crate::observables::{GitRepo, GitReposObservable, GitReposState, Observable};
use anyhow::{Context, Result};
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
        // We only read from tauri::Store once at the start
        let repos = store
            .entries()
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<GitRepo>(v.clone())
                    .ok()
                    .map(|entry| (k.to_string(), entry))
            })
            .collect();
        log::debug!("repos: {:?}", repos);
        observable.set_state(GitReposState { repos })?;
        Ok(Self {
            handle,
            store,
            observable,
        })
    }

    fn add(&self, repo: GitRepo) -> Result<()> {
        // Filesystem first
        let id = repo.slug.clone();
        let value = serde_json::to_value(repo.clone())?;
        self.store.set(id.clone(), value);
        self.save()?;

        let state = self.observable.get_state()?;
        let mut repos = state.repos.clone();
        repos.insert(id, repo);
        self.observable.set_state(GitReposState { repos })?;

        Ok(())
    }

    fn save(&self) -> Result<()> {
        self.store.save()?;
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

        self.store.delete(id.clone());
        self.save()?;

        self.observable.set_state(GitReposState { repos })?;

        let repo_path = PathBuf::from(&removed_repo.local_path);
        if repo_path.exists() {
            fs::remove_dir_all(&repo_path)
                .with_context(|| format!("Failed to remove local repo cache at {}", repo_path.display()))?;

            if let Some(parent) = repo_path.parent() {
                if parent.read_dir()?.next().is_none() {
                    let _ = fs::remove_dir(parent);
                }
            }
        }

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
}
