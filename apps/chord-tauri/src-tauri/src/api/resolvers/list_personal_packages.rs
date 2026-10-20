use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;
use crate::app::chord_package_manager::PersonalPackages;

pub async fn list_personal_packages(api: ApiImpl) -> AppResult<PersonalPackages> {
    let handle = api.handle()?;
    handle
        .app_state()
        .chord_package_manager()
        .registry
        .personal
        .list()
        .map_err(AppError::detailed)
}
