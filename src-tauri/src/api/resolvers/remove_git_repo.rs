use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn remove_git_repo(api: ApiImpl, repo: String) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.state().git_repos_store();
    store.remove_repo(&repo)?;

    let chord_pm = handle.state().chord_package_manager();
    chord_pm.reload_all().await?;
    Ok(())
}
