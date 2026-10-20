use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;

pub async fn delete_personal_chords_file(
    api: ApiImpl,
    dirname: String,
    pathslug: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let chord_pm = handle.app_state().chord_package_manager();
    chord_pm
        .registry
        .personal
        .delete_file(&dirname, &pathslug)
        .map_err(AppError::detailed)?;
    chord_pm.reload_all().await?;
    Ok(())
}
