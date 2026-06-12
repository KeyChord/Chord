use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn pick_local_chord_folder(api: ApiImpl) -> AppResult<Option<String>> {
    let handle = api.handle()?;
    let chord_pm = handle.app_state().chord_package_manager();
    Ok(chord_pm.registry
        .local
        .pick()
        .map(|folder| folder.map(|folder| folder.path().display().to_string()))?)
}
