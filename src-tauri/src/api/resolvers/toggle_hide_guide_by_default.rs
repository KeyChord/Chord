use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn toggle_hide_guide_by_default(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let settings = handle.app_state().settings();
    Ok(settings.toggle_hide_guide_by_default()?)
}
