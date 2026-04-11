use std::fs;
use tauri::AppHandle;
use crate::app::AppSingleton;
use super::{load_repos, rewrite_repos, GitReposStore};
use crate::git::GitHubRepoRef;
use crate::state::{GitReposObservable, GitReposState, Observable};

impl<T> AppSingleton<T> for GitReposStore {
    fn new(handle: AppHandle) -> Self {
        Self {
            handle,
            observable: GitReposObservable::uninitialized(),
        }
    }

    fn init(&self, observable: GitReposObservable) -> anyhow::Result<()> {
        self.observable.init(observable);

        let mut repos = load_repos(self.store()?.as_ref())?;
        let repos_root = self.github_repos_dir()?;

        let mut changed = false;
        for repo in repos.values_mut() {
            let repo_ref = GitHubRepoRef {
                owner: repo.owner.clone(),
                name: repo.name.clone(),
            };
            let expected_path = repo_ref.local_abspath(&repos_root, repo.pinned_rev.as_deref());
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
        }

        if changed {
            rewrite_repos(self.store()?.as_ref(), &repos)?;
        }

        self.observable.set_state(GitReposState { repos })?;
        Ok(())
    }
}

