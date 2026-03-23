use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Runtime, Wry};
use tauri_plugin_store::Store;
use crate::feature::SafeAppHandle;
use crate::git::{clone_repo, GitHubRepoRef};
use crate::observables::{GitRepo, GitReposObservable, GitReposState};
use crate::stores::AppHandleStoreExt;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitReposStoreEntry {
    pub repo: GitRepo
}

pub struct GitReposStore {
    pub store: Arc<Store<Wry>>,
    pub observable: Arc<GitReposObservable>,

    handle: SafeAppHandle
}

impl Clone for GitReposStore {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            observable: self.observable.clone(),
            handle: self.handle.clone()
        }
    }
}

impl GitReposStore {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let store = handle.store("repos.json")?;
        let observable = GitReposObservable::new(handle.clone())?;
        Ok(Self { handle, store, observable: Arc::new(observable) })
    }

    pub fn entries(&self) -> HashMap<String, GitReposStoreEntry> {
        // We clone it to avoid deadlocks (since .entries() calls a lock)
        self.store
            .entries()
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<GitReposStoreEntry>(v.clone())
                    .ok()
                    .map(|entry| (k.to_string(), entry))
            })
            .collect()
    }

    pub fn set(&self, shortcut: &str, entry: GitReposStoreEntry) {
        let value = serde_json::to_value(entry).unwrap();
        self.store.set(shortcut, value);
    }

    pub fn remove(&self, shortcut: &str) {
        self.store.delete(shortcut);
    }



    pub fn github_repos_dir(&self) -> Result<PathBuf> {
        let dir = self.handle.path().app_cache_dir()?;
        Ok(dir.join("repos/github.com"))
    }

    pub fn add_repo(&self, repo_ref: GitHubRepoRef) -> Result<()> {
        let repos_root = self.github_repos_dir()?;
        let repo_path = repo_ref.local_path(&repos_root);
        let state = self.observable.get_state()?;
        let mut repos = state.repos.clone();

        let repo = if repo_path.join(".git").exists() {
            repo_ref.into_repo(&repos_root)
        } else {
            clone_repo(&repo_ref, &repo_path)?;
            repo_ref.into_repo(&repos_root)
        };
        repos.push(repo);

        log::debug!("repos: {:?}", repos);
        self.observable.set_state(GitReposState { repos })?;
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
        let state = self.observable.get_state()?;
        let mut repos = state.repos.clone();
        match repos.iter_mut().find(|r| r.owner == repo.owner && r.slug == r.owner) {
            Some(existing) => *existing = repo,
            None => repos.push(repo),
        }

        self.observable.set_state(GitReposState { repos })?;
        Ok(())
    }
}
