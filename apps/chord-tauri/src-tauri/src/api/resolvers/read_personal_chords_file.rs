use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;

pub async fn read_personal_chords_file(
    api: ApiImpl,
    dirname: String,
    pathslug: String,
) -> AppResult<String> {
    let handle = api.handle()?;
    handle
        .app_state()
        .chord_package_manager()
        .registry
        .personal
        .read_file(&dirname, &pathslug)
        .map_err(AppError::detailed)
}
