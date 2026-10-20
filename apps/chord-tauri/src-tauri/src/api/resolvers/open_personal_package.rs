use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;
use tauri_plugin_opener::OpenerExt;

/// Opens a personal package's folder in Finder, or `~/.chord/packages` without one.
pub async fn open_personal_package(api: ApiImpl, dirname: Option<String>) -> AppResult<()> {
    let handle = api.handle()?;
    let personal = &handle.app_state().chord_package_manager().registry.personal;
    let path = match dirname {
        Some(dirname) => personal.package_root(&dirname),
        None => personal.packages_dir(),
    }
    .map_err(AppError::detailed)?;
    handle
        .opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|error| AppError::Message(error.to_string()))
}
