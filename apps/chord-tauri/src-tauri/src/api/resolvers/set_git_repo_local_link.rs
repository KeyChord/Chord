use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::state::GitRepo;

pub async fn set_git_repo_local_link(
    api: ApiImpl,
    repo: String,
    path: Option<String>,
) -> AppResult<GitRepo> {
    let handle = api.handle()?;
    let manager = handle.app_state().chord_package_manager();
    let repo = manager
        .registry
        .git
        .git_repos_store
        .set_local_link(&repo, path)?;
    manager.reload_all().await?;
    Ok(repo)
}
