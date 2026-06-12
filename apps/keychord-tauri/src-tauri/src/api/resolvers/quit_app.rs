use crate::api::{ApiImpl, AppResult};

pub async fn quit_app(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    handle.exit(0);
    Ok(())
}
