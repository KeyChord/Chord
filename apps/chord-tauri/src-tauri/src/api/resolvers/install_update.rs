use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn install_update(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    Ok(handle.app_state().app_updater().install().await?)
}
