use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

pub async fn list_local_chord_monorepos(api: ApiImpl) -> AppResult<Vec<String>> {
    let handle = api.handle()?;
    Ok(handle
        .app_state()
        .chord_package_manager()
        .registry
        .local
        .list_monorepos()?)
}
