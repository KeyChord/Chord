use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn pick_local_chord_folder(api: ApiImpl) -> AppResult<Option<String>> {
    let app_handle = api.handle()?;
    let chord_pm = app_handle.chord_package_manager();
    Ok(chord_pm.registry
        .local
        .pick()
        .map(|folder| folder.map(|folder| folder.path().display().to_string()))?)
}
