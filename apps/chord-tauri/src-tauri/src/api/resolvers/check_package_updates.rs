use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn check_package_updates(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let state = handle.app_state();
    state
        .chord_package_manager()
        .registry
        .git
        .git_repos_store
        .check_updates()
        .await?;
    Ok(())
}
