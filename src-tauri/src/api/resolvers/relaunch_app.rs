use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn relaunch_app(api: ApiImpl, bundle_id: String) -> AppResult<()> {
    let handle = api.handle()?;
    let desktop_app_manager = handle.desktop_app_manager();
    Ok(desktop_app_manager.relaunch_app(&bundle_id)?)
}
