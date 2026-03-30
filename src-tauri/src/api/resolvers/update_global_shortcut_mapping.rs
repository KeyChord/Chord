use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;
use crate::models::SimulatedShortcut;

pub async fn update_global_shortcut_mapping(
    api: ApiImpl,
    old_shortcut: String,
    new_shortcut: String,
) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_global_hotkey_store();
    let old_shortcut = old_shortcut.trim();
    let new_shortcut = new_shortcut.trim();

    if old_shortcut.is_empty() || new_shortcut.is_empty() {
        return Err(AppError::Message("shortcut cannot be empty".into()));
    }

    new_shortcut.parse::<SimulatedShortcut>()
        .map_err(|err| AppError::Message(format!("invalid shortcut {new_shortcut:?}: {err}")))?;

    store.update_shortcut(old_shortcut, new_shortcut)?;
    Ok(())
}
