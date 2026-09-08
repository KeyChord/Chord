use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn remove_local_chord_monorepo(api: ApiImpl, path: String) -> AppResult<()> {
    let handle = api.handle()?;
    let manager = handle.app_state().chord_package_manager();
    manager.registry.local.remove_monorepo(&path)?;
    manager.reload_all().await?;
    Ok(())
}
