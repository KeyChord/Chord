use crate::api::{ApiImpl, AppResult, GlobalShortcutMappingInfo};
use crate::app::AppHandleExt;

pub async fn list_global_shortcut_mappings(
    api: ApiImpl,
) -> AppResult<Vec<GlobalShortcutMappingInfo>> {
    let handle = api.handle()?;
    let store = handle.app_state().global_hotkey_store();
    let mut mappings = store
        .entries()?
        .into_iter()
        .map(|(shortcut, entry)| GlobalShortcutMappingInfo {
            shortcut,
            bundle_id: entry.bundle_id,
            hotkey_id: entry.hotkey_id,
        })
        .collect::<Vec<_>>();

    mappings.sort_by(|left, right| {
        left.bundle_id
            .cmp(&right.bundle_id)
            .then(left.hotkey_id.cmp(&right.hotkey_id))
            .then(left.shortcut.cmp(&right.shortcut))
    });

    Ok(mappings)
}
