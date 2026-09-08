use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn refresh_cli_installation(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    tauri::async_runtime::spawn_blocking(move || {
        handle.app_state().settings().refresh_cli_installation()
    })
    .await
    .map_err(|error| crate::api::AppError::Message(error.to_string()))??;
    Ok(())
}
