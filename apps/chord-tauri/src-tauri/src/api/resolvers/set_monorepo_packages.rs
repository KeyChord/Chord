use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn set_monorepo_packages(
    api: ApiImpl,
    source: String,
    names: Vec<String>,
) -> AppResult<()> {
    let handle = api.handle()?;
    let manager = handle.app_state().chord_package_manager();
    manager
        .registry
        .local
        .set_monorepo_packages(&source, names)?;
    manager.reload_all().await?;
    Ok(())
}
