use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::registry::LocalChordPackage;

pub async fn add_local_chord_folder(api: ApiImpl, path: String) -> AppResult<LocalChordPackage> {
    let handle = api.handle()?;
    let registry = handle.app_chord_package_registry();
    let folder_info = registry.local.add(&path)?;

    let chord_registry = handle.app_chord_registry();
    chord_registry.reload().await?;
    Ok(folder_info)
}
