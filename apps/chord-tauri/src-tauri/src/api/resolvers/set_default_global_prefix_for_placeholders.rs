use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn set_default_global_prefix_for_placeholders(
    api: ApiImpl,
    symbol: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let state = handle.app_state();
    state
        .settings()
        .set_default_global_prefix_for_placeholders(&symbol)?;
    state.chord_package_manager().reload_all().await?;
    Ok(())
}
