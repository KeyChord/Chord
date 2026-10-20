use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;

/// `expected` is the contents the editor opened, or `None` to create the file.
pub async fn save_personal_chords_file(
    api: ApiImpl,
    dirname: String,
    pathslug: String,
    contents: String,
    expected: Option<String>,
) -> AppResult<()> {
    let handle = api.handle()?;
    let chord_pm = handle.app_state().chord_package_manager();
    chord_pm
        .registry
        .personal
        .save_file(&dirname, &pathslug, &contents, expected.as_deref())
        .map_err(AppError::detailed)?;
    chord_pm.reload_all().await?;
    Ok(())
}
