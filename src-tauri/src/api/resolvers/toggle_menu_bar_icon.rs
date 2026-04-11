use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn toggle_menu_bar_icon(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let settings = handle.state().settings();
    Ok(settings.toggle_menu_bar_icon()?)
}
