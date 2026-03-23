use crate::input::{Key, KeyEvent};
use crate::mode::AppMode;
use anyhow::Result;
use keycode::KeyMappingCode;
use tauri::AppHandle;
use crate::feature::app_handle_ext::AppHandleExt;

pub fn handle_key_event(handle: AppHandle, key_event: KeyEvent) -> Result<()> {
    let app_mode = handle.app_context().get_app_mode();
    let chorder = handle.app_chorder();

    match app_mode {
        AppMode::Chord => {
                chorder
                .handle_key_event(handle.clone(), &key_event)?;
        }
        AppMode::None => {
            let should_finalize_chord_mode =
                matches!(key_event, KeyEvent::Release(Key(KeyMappingCode::Space)))
                    && chorder.observable.state.get()?.is_idle();

            if should_finalize_chord_mode {
                    chorder
                    .handle_key_event(handle.clone(), &key_event)?;
            }

            chorder.ensure_inactive()?;
        }
    }

    Ok(())
}
