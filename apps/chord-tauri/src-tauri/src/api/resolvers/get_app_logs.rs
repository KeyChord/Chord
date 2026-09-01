use crate::api::{ApiImpl, AppResult};
use crate::logging::{AppLogEntry, recent_entries};

pub async fn get_app_logs(_api: ApiImpl) -> AppResult<Vec<AppLogEntry>> {
    Ok(recent_entries())
}
