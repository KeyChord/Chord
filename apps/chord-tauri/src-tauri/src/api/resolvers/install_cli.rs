use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn install_cli(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    tauri::async_runtime::spawn_blocking(move || handle.app_state().settings().install_cli())
        .await
        .map_err(|error| crate::api::AppError::Message(error.to_string()))??;
    Ok(())
}
