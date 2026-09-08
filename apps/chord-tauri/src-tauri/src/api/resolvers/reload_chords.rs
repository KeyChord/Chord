use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn reload_chords(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    handle
        .app_state()
        .chord_package_manager()
        .reload_all()
        .await?;
    Ok(())
}
