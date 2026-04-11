use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::git::GitHubRepoRef;
use crate::state::GitRepo;

pub async fn add_git_repo(api: ApiImpl, repo: String) -> AppResult<GitRepo> {
    let handle = api.handle()?;
    let store = handle.state().git_repos_store();
    let repo_ref = GitHubRepoRef::parse(&repo)?;
    store.add_repo(repo_ref.clone())?;

    let chord_pm = handle.state().chord_package_manager();
    chord_pm.reload_all().await?;
    Ok(repo_ref.into_repo(store.github_repos_dir()?.as_path()))
}
