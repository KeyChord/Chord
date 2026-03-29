use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::git::GitHubRepoRef;
use crate::observables::GitRepo;

pub async fn sync_git_repo(api: ApiImpl, repo: String) -> AppResult<GitRepo> {
    let handle = api.handle()?;
    let store = handle.app_git_repos_store();
    let repo_ref = GitHubRepoRef::parse(&repo)?;
    store.sync_repo(repo_ref.clone())?;

    let chord_package_manager = handle.chord_package_manager();
    chord_package_manager.re().await?;
    Ok(repo_ref.into_repo(store.github_repos_dir()?.as_path()))
}
