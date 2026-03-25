use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;

pub async fn remove_global_shortcut_mapping(api: ApiImpl, shortcut: String) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_global_hotkey_store();
    let trimmed_shortcut = shortcut.trim();
    if trimmed_shortcut.is_empty() {
        return Err(AppError::Message("cannot be empty".into()));
    }

    store.remove(trimmed_shortcut)?;
    Ok(())
}
