use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;

/// Adds a chord that opens an app from anywhere. The bundle ID keeps the command valid when
/// an app's name has spaces or quotes in it.
pub async fn add_app_chord(
    api: ApiImpl,
    sequence: String,
    name: String,
    bundle_id: String,
) -> AppResult<()> {
    let is_bundle_id = !bundle_id.is_empty()
        && bundle_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if !is_bundle_id {
        return Err(AppError::Message(format!("{bundle_id} is not a bundle ID")));
    }
    let handle = api.handle()?;
    let chord_pm = handle.app_state().chord_package_manager();
    chord_pm
        .registry
        .personal
        .add_global_chord(&sequence, &name, &format!("open -b {bundle_id}"))
        .map_err(AppError::detailed)?;
    chord_pm.reload_all().await?;
    Ok(())
}
