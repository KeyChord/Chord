use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::placeholder_chord_store::PlaceholderChordStoreKey;

pub async fn remove_placeholder_chord_binding(
    api: ApiImpl,
    file_path: String,
    sequence_template: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_state().placeholder_chord_store();
    store.remove(&PlaceholderChordStoreKey {
        file_path,
        sequence_template,
    })?;
    handle.app_state().chord_package_manager().reload_all().await?;

    Ok(())
}
