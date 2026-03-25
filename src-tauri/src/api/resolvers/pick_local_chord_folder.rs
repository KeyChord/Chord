use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn pick_local_chord_folder(api: ApiImpl) -> AppResult<Option<String>> {
    let app_handle = api.handle()?;
    let registry = app_handle.app_chord_package_registry();
    Ok(registry
        .local
        .pick()
        .map(|folder| folder.map(|folder| folder.path().display().to_string()))?)
}
