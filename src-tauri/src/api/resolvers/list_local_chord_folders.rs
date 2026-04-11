use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn list_local_chord_folders(api: ApiImpl) -> AppResult<Vec<String>> {
    let handle = api.handle()?;
    let chord_pm = handle.state().chord_package_manager();
    Ok(chord_pm.registry.local.list_package_paths()?.iter().map(|r| r.to_string_lossy().to_string()).collect())
}
