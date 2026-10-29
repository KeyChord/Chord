use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn set_package_approvals(
    api: ApiImpl,
    identities: Vec<String>,
    approved: bool,
) -> AppResult<()> {
    api.handle()?
        .app_state()
        .chord_package_manager()
        .set_package_approvals(&identities, approved)
        .await?;
    Ok(())
}
