use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn refresh_permissions(api: ApiImpl) -> AppResult<(bool, bool)> {
    let handle = api.handle()?;
    Ok(handle.app_state().permissions().load().await?)
}
