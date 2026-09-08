use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn add_local_chord_monorepo(api: ApiImpl, path: String) -> AppResult<()> {
    let handle = api.handle()?;
    let manager = handle.app_state().chord_package_manager();
    manager.registry.local.add_monorepo(&path)?;
    manager.reload_all().await?;
    Ok(())
}
