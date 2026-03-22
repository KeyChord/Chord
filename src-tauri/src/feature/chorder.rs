use super::ChorderIndicatorPanel;
use crate::chords::{press_chord, release_chord, Chord, ChordPayload};
use crate::input::Key;
use crate::{input::KeyEvent, AppContext};
use anyhow::Result;
use device_query::DeviceQuery;
use keycode::KeyMappingCode;
use keycode::KeyMappingCode::*;
use observable_property::ObservableProperty;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

const CHORDER_INDICATOR_STATE_CHANGED_EVENT: &str = "chorder-indicator-state-changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChorderIndicatorStatePayload {
    visible: bool,
    buffer_keys: Vec<String>,
    active_keys: Vec<String>,
    shift_pressed: bool,
}

pub struct Chorder {
    pub state: ObservableProperty<Arc<ChorderState>>,
    panel: ChorderIndicatorPanel,
}

impl Chorder {
    pub fn new(panel: ChorderIndicatorPanel) -> Self {
        Self {
            state: ObservableProperty::new(Arc::new(ChorderState::new())),
            panel,
        }
    }

    pub fn ensure_active(&self, handle: AppHandle) -> Result<()> {
        self.panel.ensure_visible(handle.clone())?;
        let state = self.state.get()?;
        self.emit_indicator_state(&handle, true, state.as_ref())?;
        Ok(())
    }

    pub fn ensure_inactive(&self, handle: AppHandle) -> Result<()> {
        let state = self.state.get()?;
        if !self.panel.is_visible() && state.is_idle() {
            return Ok(());
        }

        let next_state = ChorderState::new();
        self.state.set(Arc::new(next_state.clone()))?;
        self.panel.ensure_hidden(handle.clone())?;
        self.emit_indicator_state(&handle, false, &next_state)?;
        Ok(())
    }

    // If `handle_key_event` is called, the state is guaranteed to be active
    pub fn handle_key_event(&self, handle: AppHandle, key_event: &KeyEvent) -> Result<()> {
        // Don't handle any modifier key events
        let modifiers = Key::modifiers();
        let (KeyEvent::Press(key) | KeyEvent::Release(key)) = key_event;
        if modifiers.contains(key) {
            log::debug!("Ignoring modifier key: {:?}", key);
            return Ok(());
        }

        let non_shift_modifiers = Key::non_shift_modifiers();
        let context = handle.state::<AppContext>();
        let Some(device_state) = &context.device_state else {
            log::debug!("no accessibility permissions");
            return Ok(());
        };
        let device_keys = device_state.get_keys();

        // If any non-Shift modifier keys are held down, do not handle the event, because it's
        // likely the user just wants to execute a regular shortcut
        if device_keys
            .iter()
            .copied()
            .any(|key| non_shift_modifiers.contains(&key.into()))
        {
            log::debug!(
                "Ignoring event because the following modifiers were held down: {:?}",
                device_keys
            );
            return Ok(());
        }

        match key_event {
            KeyEvent::Release(Key(code)) => {
                if let Some(pressed_chord) = &self.state.get()?.pressed_chord {
                    if code == &KeyMappingCode::CapsLock {
                        release_chord(handle.clone(), pressed_chord)?;
                    } else if pressed_chord.keys.last().is_some_and(|k| &k.0 == code) {
                        release_chord(handle.clone(), pressed_chord)?;
                    }
                }

                if code == &KeyMappingCode::Space {
                    self.replace_state(&handle, ChorderState::new())?;
                }

                Ok(())
            }

            // If the caps lock key is pressed, it means we should execute (and clear) the chord
            // currently in `key_buffer`, or if empty, execute the last chord
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                self.ensure_active(handle.clone())?;

                let context = handle.state::<AppContext>();
                let loaded_app_chords = context.loaded_app_chords.read();
                let state = self.state.get()?;
                let key_buffer = state.key_buffer.clone();

                // An empty `key_buffer` means we should execute the last executed chord
                if key_buffer.is_empty() {
                    // If there isn't an active chord, then do nothing
                    let Some(last_chord) = &state.active_chord else {
                        log::error!("Key buffer is empty and no chord is active");
                        return Ok(());
                    };

                    let application_id = context.frontmost_application_id.load().as_ref().clone();
                    let chord_runtime =
                        loaded_app_chords.get_chord_runtime(&last_chord.keys, application_id);
                    if let Some(chord_runtime) = chord_runtime {
                        press_chord(
                            handle.clone(),
                            chord_runtime,
                            &ChordPayload {
                                chord: last_chord.clone(),
                                num_times: 1,
                            },
                        )?;
                        self.replace_state(
                            &handle,
                            ChorderState {
                            pressed_chord: state.active_chord.clone(),
                            key_buffer: vec![],
                            active_chord: state.active_chord.clone(),
                        },
                        )?;
                    } else {
                        // e.g. we ran it on a different app
                        log::error!("Last chord no longer applies");
                        self.replace_state(
                            &handle,
                            ChorderState {
                            key_buffer: vec![],
                            pressed_chord: None,
                            active_chord: None,
                        },
                        )?;
                    }

                    return Ok(());
                }

                // A non-empty key_buffer means we should execute the chord.
                log::debug!("Executing key_buffer {:?}", key_buffer);

                let Some(chord_runtime) = loaded_app_chords.get_chord_runtime(
                    &state.key_buffer,
                    context.frontmost_application_id.load().as_ref().clone(),
                ) else {
                    log::error!(
                        "Missing chord runtime for chord {:?} in application: {:?}",
                        state.key_buffer,
                        context.frontmost_application_id.load().as_ref().clone()
                    );
                    return Ok(());
                };

                let Some(chord_payload) = chord_runtime.get_chord(&key_buffer) else {
                    // If the chord is the buffer is invalid, reset it
                    log::error!(
                        "Invalid chord {:?} in application: {:?}",
                        state.key_buffer,
                        context.frontmost_application_id.load().as_ref().clone()
                    );
                    self.replace_state(
                        &handle,
                        ChorderState {
                        key_buffer: vec![],
                        pressed_chord: None,
                        active_chord: None,
                    },
                    )?;
                    return Ok(());
                };

                press_chord(handle.clone(), &chord_runtime, &chord_payload)?;
                self.replace_state(
                    &handle,
                    ChorderState {
                    pressed_chord: Some(chord_payload.chord.clone()),
                    key_buffer: vec![],
                    active_chord: Some(chord_payload.chord.clone()),
                },
                )?;
                Ok(())
            }
            KeyEvent::Press(key) => {
                // Ignore space presses
                if key == &Key(KeyMappingCode::Space) {
                    self.ensure_active(handle)?;
                    return Ok(());
                }

                self.ensure_active(handle.clone())?;
                let is_shift_pressed = context.is_shift_pressed();
                if is_shift_pressed {
                    self.handle_shifted_key_press(handle, key)
                } else {
                    self.handle_unshifted_key_press(handle, key)
                }
            }
        }
    }

    // If an unshifted key is pressed, we append it to the key buffer, which always clears
    // our `active_chord`
    fn handle_unshifted_key_press(&self, handle: AppHandle, key: &Key) -> Result<()> {
        let state = self.state.get()?;
        let mut next_key_buffer = state.key_buffer.clone();
        next_key_buffer.push(key.clone());
        log::debug!("New key buffer: {:?}", next_key_buffer);
        self.replace_state(
            &handle,
            ChorderState {
            key_buffer: next_key_buffer,
            pressed_chord: None,
            active_chord: None,
        },
        )?;
        Ok(())
    }

    // If shift is pressed, it means the user is trying to execute a chord.
    // If a chord is executed, we always reset `key_buffer`.
    fn handle_shifted_key_press(&self, handle: AppHandle, key: &Key) -> Result<()> {
        let context = handle.state::<AppContext>();
        let state = self.state.get()?;
        let key_buffer = state.key_buffer.clone();

        let sequence = {
            // If key_buffer is empty (i.e. we just activated a chord), we should use that chord to
            // determine our sequence
            if key_buffer.is_empty() {
                let Some(active_chord) = &state.active_chord else {
                    // If `key_buffer` and `active_chord` is empty, then we do nothing
                    log::error!("No chord active");
                    return Ok(());
                };

                let mut new_chord = active_chord.keys.clone();
                new_chord.pop();
                new_chord.push(key.clone());
                new_chord
            }
            // If `key_buffer` is non-empty, we should run the chord `key_buffer` + key
            else {
                let mut sequence = key_buffer.clone();
                sequence.push(key.clone());
                sequence
            }
        };

        let frontmost_application_id = context.frontmost_application_id.load().as_ref().clone();
        let loaded_app_chords = context.loaded_app_chords.read();
        let chord_runtime =
            loaded_app_chords.get_chord_runtime(&sequence, frontmost_application_id);
        let (Some(chord_runtime), Some(chord_payload)) = (
            chord_runtime,
            chord_runtime.and_then(|r| r.get_chord(&sequence)),
        ) else {
            // We don't change the state for an invalid sequence
            log::debug!("Invalid sequence {:?}", sequence);
            return Ok(());
        };

        log::debug!("Pressing chord: {:?}", chord_payload);
        press_chord(handle.clone(), &chord_runtime, &chord_payload)?;
        self.replace_state(
            &handle,
            ChorderState {
            // We always clear the key_buffer if a chord is pressed
            key_buffer: vec![],
            pressed_chord: Some(chord_payload.chord.clone()),
            active_chord: Some(chord_payload.chord.clone()),
        },
        )?;

        Ok(())
    }

    fn replace_state(&self, handle: &AppHandle, next_state: ChorderState) -> Result<()> {
        self.state.set(Arc::new(next_state.clone()))?;
        self.emit_indicator_state(handle, self.panel.is_visible(), &next_state)?;
        Ok(())
    }

    fn emit_indicator_state(
        &self,
        handle: &AppHandle,
        visible: bool,
        state: &ChorderState,
    ) -> Result<()> {
        let payload = ChorderIndicatorStatePayload {
            visible,
            buffer_keys: format_keys(&state.key_buffer),
            active_keys: state
                .active_chord
                .as_ref()
                .map(|chord| format_keys(&chord.keys))
                .unwrap_or_default(),
            shift_pressed: handle.state::<AppContext>().is_shift_pressed(),
        };

        if let Some(window) = handle.get_webview_window(crate::constants::INDICATOR_WINDOW_LABEL) {
            window.emit(CHORDER_INDICATOR_STATE_CHANGED_EVENT, payload)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChorderState {
    // The key buffer represents the pending letters for a not-yet created chord.
    // When a chord is executed, the key buffer is always cleared.
    pub key_buffer: Vec<Key>,

    // The chord currently being pressed down.
    pub pressed_chord: Option<Chord>,

    // The chord that is "active"
    pub active_chord: Option<Chord>,
}

impl ChorderState {
    pub fn new() -> Self {
        Self {
            key_buffer: Vec::new(),
            pressed_chord: None,
            active_chord: None,
        }
    }

    pub fn is_idle(&self) -> bool {
        self.key_buffer.is_empty() && self.pressed_chord.is_none() && self.active_chord.is_none()
    }
}

fn format_keys(keys: &[Key]) -> Vec<String> {
    keys.iter().map(|key| format_key(*key)).collect()
}

fn format_key(key: Key) -> String {
    if let Some(ch) = key.to_char(false) {
        return ch.to_ascii_uppercase().to_string();
    }

    match key.0 {
        ShiftLeft | ShiftRight => "Shift".to_string(),
        ControlLeft | ControlRight => "Ctrl".to_string(),
        MetaLeft | MetaRight => "Cmd".to_string(),
        AltLeft | AltRight => "Alt".to_string(),
        CapsLock => "Caps".to_string(),
        Space => "Space".to_string(),
        Enter => "Enter".to_string(),
        Tab => "Tab".to_string(),
        Escape => "Esc".to_string(),
        ArrowUp => "Up".to_string(),
        ArrowDown => "Down".to_string(),
        ArrowLeft => "Left".to_string(),
        ArrowRight => "Right".to_string(),
        Backspace => "Backspace".to_string(),
        Delete => "Delete".to_string(),
        Home => "Home".to_string(),
        End => "End".to_string(),
        PageUp => "PgUp".to_string(),
        PageDown => "PgDn".to_string(),
        Fn => "Fn".to_string(),
        other => format!("{other:?}"),
    }
}
