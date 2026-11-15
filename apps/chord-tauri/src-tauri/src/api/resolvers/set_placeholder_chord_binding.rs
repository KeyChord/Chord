use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::placeholder_chord_store::{
    PlaceholderChordStoreEntry, PlaceholderChordStoreKey, normalize_placeholder_sequence,
};

/// Saves the user's value for `<placeholder>` in one package's chords file and reloads, so its
/// chords use the new keys.
pub async fn set_placeholder_chord_binding(
    api: ApiImpl,
    package_name: String,
    pathslug: String,
    placeholder: String,
    sequence: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_state().placeholder_chord_store();
    let key = PlaceholderChordStoreKey {
        package: package_name,
        file: pathslug,
        placeholder,
    };
    let entry = PlaceholderChordStoreEntry {
        sequence: normalize_placeholder_sequence(&sequence)?,
    };

    store.set(key, entry)?;
    let chord_pm = handle.app_state().chord_package_manager();
    chord_pm.reload_all().await?;

    Ok(())
}
