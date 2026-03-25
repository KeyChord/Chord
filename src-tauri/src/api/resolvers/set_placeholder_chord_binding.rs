use crate::api::{ApiImpl, AppResult, normalize_placeholder_sequence};
use crate::app::AppHandleExt;
use crate::app::placeholder_chord_store::{PlaceholderChordStoreEntry, PlaceholderChordStoreKey};

pub async fn set_placeholder_chord_binding(
    api: ApiImpl,
    file_path: String,
    sequence_template: String,
    sequence: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_placeholder_chord_store();
    let key = PlaceholderChordStoreKey {
        file_path,
        sequence_template,
    };
    let entry = PlaceholderChordStoreEntry {
        sequence: normalize_placeholder_sequence(&sequence)?,
    };

    store.set(key, entry)?;
    handle.app_chord_registry().reload().await?;
    Ok(())
}
