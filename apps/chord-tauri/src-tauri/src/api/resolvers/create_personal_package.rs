use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;

pub async fn create_personal_package(api: ApiImpl, dirname: String) -> AppResult<()> {
    let handle = api.handle()?;
    let chord_pm = handle.app_state().chord_package_manager();
    chord_pm
        .registry
        .personal
        .create_package(&dirname)
        .map_err(AppError::detailed)?;
    chord_pm.reload_all().await?;
    Ok(())
}
