use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::git::GitHubRepoRef;

pub async fn remove_git_repo(api: ApiImpl, repo: String) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_git_repos_store();
    let repo_ref = GitHubRepoRef::parse(&repo)?;
    store.remove_repo(&repo_ref)?;

    let chord_registry = handle.app_chord_registry();
    chord_registry.reload().await?;
    Ok(())
}
