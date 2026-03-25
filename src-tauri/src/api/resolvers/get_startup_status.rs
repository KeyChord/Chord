use crate::api::{ApiImpl, AppResult};
use crate::tauri_app::startup;
use crate::tauri_app::startup::StartupStatusInfo;

pub async fn get_startup_status(api: ApiImpl) -> AppResult<StartupStatusInfo> {
    let handle = api.handle()?;
    Ok(startup::get_startup_status(&handle)?)
}
