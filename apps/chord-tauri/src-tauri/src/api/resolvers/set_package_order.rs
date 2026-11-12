use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn set_package_order(api: ApiImpl, order: Vec<String>) -> AppResult<()> {
    let handle = api.handle()?;
    handle
        .app_state()
        .chord_package_manager()
        .set_package_order(order)
        .await?;
    Ok(())
}
