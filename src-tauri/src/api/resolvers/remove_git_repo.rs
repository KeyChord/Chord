use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn remove_git_repo(api: ApiImpl, repo: String) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_git_repos_store();
    store.remove_repo(&repo)?;

    let chord_registry = handle.app_chord_registry();
    chord_registry.reload().await?;
    Ok(())
}
