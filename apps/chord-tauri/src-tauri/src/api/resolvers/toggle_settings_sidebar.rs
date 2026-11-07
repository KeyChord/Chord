use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn toggle_settings_sidebar(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let settings = handle.app_state().settings();
    Ok(settings.toggle_settings_sidebar()?)
}
