use crate::app::AppHandleExt;
use crate::input::{Key, KeyEvent, emit_caps_lock};
use crate::mode::AppMode;
use crate::observables::{ChorderObservable, ChorderState};
use anyhow::Result;
use keycode::KeyMappingCode;
use tauri::AppHandle;

pub fn handle_key_event(handle: AppHandle, key_event: KeyEvent) -> Result<()> {
    let app_mode = handle.app_context().get_app_mode();
    let chorder = handle.app_chorder();

    match app_mode {
        AppMode::Chord => {
            chorder.handle_key_event(&key_event)?;
        }
        AppMode::None => {
            let should_emit_caps_lock = handle
                .app_context()
                .take_caps_lock_passthrough_on_release(&key_event);
            let state = handle.observable_state::<ChorderObservable>()?;
            let should_finalize_chord_mode = should_finalize_chord_mode(&key_event, &state);

            if should_finalize_chord_mode {
                chorder.handle_key_event(&key_event)?;
            }

            chorder.ensure_inactive()?;

            if should_emit_caps_lock {
                emit_caps_lock()?;
            }
        }
    }

    Ok(())
}

fn should_finalize_chord_mode(key_event: &KeyEvent, state: &ChorderState) -> bool {
    matches!(key_event, KeyEvent::Release(Key(KeyMappingCode::Space))) && !state.is_idle()
}
