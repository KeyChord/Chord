use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn set_onboarding_complete(api: ApiImpl, complete: bool) -> AppResult<()> {
    let handle = api.handle()?;
    Ok(handle
        .app_state()
        .settings()
        .set_onboarding_complete(complete)?)
}
