use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn toggle_autostart(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let permissions = handle.app_state().permissions();
    Ok(permissions.toggle_autostart()?)
}
