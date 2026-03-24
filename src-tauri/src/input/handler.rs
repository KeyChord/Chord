use crate::feature::app_handle_ext::AppHandleExt;
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
            chorder.handle_key_event(handle.clone(), &key_event)?;
        }
        AppMode::None => {
            let should_emit_caps_lock = handle
                .app_context()
                .take_caps_lock_passthrough_on_release(&key_event);
            let state = handle.observable_state::<ChorderObservable>()?;
            let should_finalize_chord_mode = should_finalize_chord_mode(&key_event, &state);

            if should_finalize_chord_mode {
                chorder.handle_key_event(handle.clone(), &key_event)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chords::Chord;
    use keycode::KeyMappingCode::*;

    fn test_chord() -> Chord {
        Chord {
            keys: vec![Key(KeyA)],
            name: "Test".to_string(),
            shortcut: None,
            shell: None,
            js: None,
        }
    }

    #[test]
    fn finalizes_space_release_when_key_buffer_is_pending() {
        let state = ChorderState {
            key_buffer: vec![Key(KeyA)],
            pressed_chord: None,
            active_chord: None,
        };

        assert!(should_finalize_chord_mode(
            &KeyEvent::Release(Key(KeyMappingCode::Space)),
            &state
        ));
    }

    #[test]
    fn finalizes_space_release_when_chord_state_is_still_active() {
        let state = ChorderState {
            key_buffer: vec![],
            pressed_chord: Some(test_chord()),
            active_chord: Some(test_chord()),
        };

        assert!(should_finalize_chord_mode(
            &KeyEvent::Release(Key(KeyMappingCode::Space)),
            &state
        ));
    }

    #[test]
    fn ignores_idle_or_non_space_release_events() {
        assert!(!should_finalize_chord_mode(
            &KeyEvent::Release(Key(KeyMappingCode::Space)),
            &ChorderState::default()
        ));

        assert!(!should_finalize_chord_mode(
            &KeyEvent::Press(Key(KeyA)),
            &ChorderState {
                key_buffer: vec![Key(KeyA)],
                pressed_chord: None,
                active_chord: None,
            }
        ));
    }
}
