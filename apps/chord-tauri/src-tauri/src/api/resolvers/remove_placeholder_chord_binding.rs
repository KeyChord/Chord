use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::placeholder_chord_store::PlaceholderChordStoreKey;
use crate::models::legacy_placeholder_name;
use std::path::Path;

/// Forgets the user's value for `<placeholder>`, so the file's default applies again.
pub async fn remove_placeholder_chord_binding(
    api: ApiImpl,
    package_name: String,
    pathslug: String,
    placeholder: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let app_state = handle.app_state();
    let manager = app_state.chord_package_manager();
    let package = manager.get_package_by_name(&package_name);
    let legacy_name = package
        .as_ref()
        .and_then(|package| package.compiled_chords_files.get(Path::new(&pathslug)))
        .and_then(|file| {
            let declares_named_p = file.placeholders.iter().any(|item| item.name == "<p>");
            legacy_placeholder_name(&placeholder, declares_named_p)
        });
    app_state.placeholder_chord_store().remove(
        PlaceholderChordStoreKey {
            package: package_name,
            file: pathslug,
            placeholder: placeholder.clone(),
        },
        legacy_name,
    )?;
    manager.reload_all().await?;

    Ok(())
}
