use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::chord_package_registry::LocalChordPackage;

pub async fn list_local_chord_folders(api: ApiImpl) -> AppResult<Vec<LocalChordPackage>> {
    let handle = api.handle()?;
    let registry = handle.app_chord_package_registry();
    Ok(registry.local.list()?)
}
