use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::placeholder_chord_store::PlaceholderChordStoreKey;

/// Forgets the user's value for `<placeholder>`, so the file's default applies again.
pub async fn remove_placeholder_chord_binding(
    api: ApiImpl,
    package_name: String,
    pathslug: String,
    placeholder: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_state().placeholder_chord_store();
    store.remove(&PlaceholderChordStoreKey {
        package: package_name,
        file: pathslug,
        placeholder,
    })?;
    handle.app_state().chord_package_manager().reload_all().await?;

    Ok(())
}
