use crate::api::{ApiImpl, AppResult};
use crate::tauri_app::startup;

pub async fn complete_onboarding(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    Ok(startup::complete_onboarding(&handle)?)
}
