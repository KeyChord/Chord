use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::settings::install_location;

pub async fn move_to_applications(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    install_location::move_to_applications_and_relaunch(&handle).map_err(AppError::detailed)?;
    Ok(())
}
