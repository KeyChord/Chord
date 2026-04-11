use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::chord_package_manager::chord_package_registry::LocalChordPackage;

pub async fn add_local_chord_folder(api: ApiImpl, path: String) -> AppResult<LocalChordPackage> {
    let handle = api.handle()?;
    let chord_pm = handle.state().chord_package_manager();

    let folder_info = chord_pm.registry.local.add(&path)?;
    chord_pm.reload_all().await?;
    Ok(folder_info)
}
