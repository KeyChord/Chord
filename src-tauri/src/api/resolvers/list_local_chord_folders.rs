use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn list_local_chord_folders(api: ApiImpl) -> AppResult<Vec<String>> {
    let handle = api.handle()?;
    let registry = handle.app_chord_package_registry();
    Ok(registry.local.list_package_paths()?.iter().map(|r| r.to_string_lossy().to_string()).collect())
}
