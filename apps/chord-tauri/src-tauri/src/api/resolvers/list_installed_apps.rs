use crate::api::{ApiImpl, AppError, AppResult, InstalledApp};
use crate::app::desktop_app::installed_apps;

pub async fn list_installed_apps(_api: ApiImpl) -> AppResult<Vec<InstalledApp>> {
    tauri::async_runtime::spawn_blocking(installed_apps)
        .await
        .map_err(|error| AppError::Message(error.to_string()))
}
