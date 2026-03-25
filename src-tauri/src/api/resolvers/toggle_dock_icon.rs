use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn toggle_dock_icon(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let settings = handle.app_settings();
    Ok(settings.toggle_dock_icon()?)
}
