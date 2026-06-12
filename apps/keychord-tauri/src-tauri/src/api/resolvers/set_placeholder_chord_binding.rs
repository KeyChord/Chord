use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::placeholder_chord_store::{
    PlaceholderChordStoreEntry, PlaceholderChordStoreKey, normalize_placeholder_sequence,
};

pub async fn set_placeholder_chord_binding(
    api: ApiImpl,
    file_path: String,
    sequence_template: String,
    sequence: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_state().placeholder_chord_store();
    let key = PlaceholderChordStoreKey {
        file_path,
        sequence_template,
    };
    let entry = PlaceholderChordStoreEntry {
        sequence: normalize_placeholder_sequence(&sequence)?,
    };

    store.set(key, entry)?;
    let chord_pm = handle.app_state().chord_package_manager();
    chord_pm.reload_all().await?;
    
    Ok(())
}
