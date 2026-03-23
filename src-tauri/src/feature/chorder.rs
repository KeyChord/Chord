use super::{ChorderIndicatorUi, NativeSurfaceRect};
use crate::chords::{press_chord, release_chord, Chord, ChordPayload};
use crate::input::Key;
use crate::{input::KeyEvent, AppContext};
use anyhow::Result;
use device_query::DeviceQuery;
use keycode::KeyMappingCode;
use keycode::KeyMappingCode::*;
use observable_property::ObservableProperty;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, Manager};
use typeshare::typeshare;

pub struct Chorder {
    pub state: ObservableProperty<Arc<ChorderState>>,
    pub ui: ChorderIndicatorUi,
    held_keys: Mutex<HashSet<Key>>,
}

impl Chorder {
    fn emit_will_show(&self) {
        if let Err(error) = self.ui.window.emit("chorder-will-show", ()) {
            log::error!("Failed to emit chorder will-show event: {error}");
        }
    }

    fn emit_visibility_changed(&self, visible: bool) {
        if let Err(error) = self.ui.window.emit("chorder-visibility-changed", visible) {
            log::error!("Failed to emit chorder visibility change: {error}");
        }
    }

    fn prepare_surface_before_reveal(&self, handle: AppHandle) {
        let (tx, rx) = mpsc::sync_channel(1);
        let surface_window = self.ui.window.clone();
        let surface_handle = handle.clone();
        self.ui.window.once("chorder-surface-rect", move |event| {
            match serde_json::from_str::<NativeSurfaceRect>(event.payload()) {
                Ok(rect) => {
                    if let Err(error) = ChorderIndicatorUi::configure_window_surface(
                        &surface_window,
                        surface_handle.clone(),
                        rect,
                    ) {
                        log::error!("Failed to configure native chorder surface: {error}");
                    }
                }
                Err(error) => {
                    log::error!("Failed to parse chorder surface rect: {error}");
                }
            }
        });
        self.ui.window.once("chorder-surface-ready", move |_| {
            let _ = tx.send(());
        });
        self.emit_will_show();

        if rx.recv_timeout(Duration::from_millis(160)).is_err() {
            log::debug!("Timed out waiting for chorder surface to prepare before reveal");
        }
    }

    pub fn new(ui: ChorderIndicatorUi) -> Self {
        let state = ObservableProperty::new(Arc::new(ChorderState::new()));

        let window = ui.window.clone();
        if let Err(e) = state.subscribe(Arc::new(move |_, new_state| {
            if let Err(e) = window.emit("chorder-state-changed", new_state) {
                log::error!("Failed to emit chorder state change: {e}");
            }
        })) {
            log::error!("Failed to subscribe chorder state observer: {e}");
        };

        if let Err(e) = state.set(Arc::new(ChorderState::new())) {
            log::error!("Failed to trigger initial state change: {e}");
        }

        Self {
            state,
            ui,
            held_keys: Mutex::new(HashSet::new()),
        }
    }

    pub fn ensure_active(&self, handle: AppHandle) -> Result<()> {
        if self.ui.ensure_visible(handle.clone())? {
            self.prepare_surface_before_reveal(handle.clone());
            self.ui.reveal(handle.clone())?;
            self.emit_visibility_changed(true);
        }
        Ok(())
    }

    pub fn ensure_inactive(&self, handle: AppHandle) -> Result<()> {
        self.held_keys.lock().clear();

        let state = self.state.get()?;
        if !self.ui.is_visible() && state.is_idle() {
            return Ok(());
        }

        self.state.set(Arc::new(ChorderState::new()))?;
        if self.ui.ensure_hidden(handle.clone())? {
            self.emit_visibility_changed(false);
        }
        Ok(())
    }

    // If `handle_key_event` is called, the state is guaranteed to be active
    pub fn handle_key_event(&self, handle: AppHandle, key_event: &KeyEvent) -> Result<()> {
        if !self.track_held_key_event(key_event) {
            log::debug!("Ignoring repeated chord-mode key press: {:?}", key_event);
            return Ok(());
        }

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
                let state = self.state.get()?;

                if let Some(pressed_chord) = &state.pressed_chord {
                    if code == &KeyMappingCode::CapsLock {
                        release_chord(handle.clone(), pressed_chord)?;
                    } else if pressed_chord.keys.last().is_some_and(|k| &k.0 == code) {
                        release_chord(handle.clone(), pressed_chord)?;
                    }
                }

                if code == &KeyMappingCode::Space {
                    if Self::should_execute_key_buffer_on_space_release(&state) {
                        let _ = self.execute_key_buffer(handle.clone(), state.key_buffer.clone(), true)?;
                    }

                    self.state.set(Arc::new(ChorderState::new()))?;
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
                        self.state.set(Arc::new(ChorderState {
                            pressed_chord: state.active_chord.clone(),
                            key_buffer: vec![],
                            active_chord: state.active_chord.clone(),
                        }))?;
                    } else {
                        // e.g. we ran it on a different app
                        log::error!("Last chord no longer applies");
                        self.state.set(Arc::new(ChorderState {
                            key_buffer: vec![],
                            pressed_chord: None,
                            active_chord: None,
                        }))?;
                    }

                    return Ok(());
                }

                // A non-empty key_buffer means we should execute the chord.
                log::debug!("Executing key_buffer {:?}", key_buffer);
                let Some(chord) = self.execute_key_buffer(handle.clone(), key_buffer.clone(), false)?
                else {
                    self.state.set(Arc::new(ChorderState {
                        key_buffer: vec![],
                        pressed_chord: None,
                        active_chord: None,
                    }))?;
                    return Ok(());
                };

                self.state.set(Arc::new(ChorderState {
                    pressed_chord: Some(chord.clone()),
                    key_buffer: vec![],
                    active_chord: Some(chord),
                }))?;
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

    fn track_held_key_event(&self, key_event: &KeyEvent) -> bool {
        let mut held_keys = self.held_keys.lock();
        should_handle_held_key_event(&mut held_keys, key_event)
    }

    fn execute_key_buffer(
        &self,
        handle: AppHandle,
        key_buffer: Vec<Key>,
        release_immediately: bool,
    ) -> Result<Option<Chord>> {
        let context = handle.state::<AppContext>();
        let frontmost_application_id = context.frontmost_application_id.load().as_ref().clone();
        let loaded_app_chords = context.loaded_app_chords.read();
        let Some(chord_runtime) =
            loaded_app_chords.get_chord_runtime(&key_buffer, frontmost_application_id.clone())
        else {
            log::error!(
                "Missing chord runtime for chord {:?} in application: {:?}",
                key_buffer,
                frontmost_application_id
            );
            return Ok(None);
        };

        let Some(chord_payload) = chord_runtime.get_chord(&key_buffer) else {
            log::error!(
                "Invalid chord {:?} in application: {:?}",
                key_buffer,
                frontmost_application_id
            );
            return Ok(None);
        };

        press_chord(handle.clone(), &chord_runtime, &chord_payload)?;

        if release_immediately {
            release_chord(handle.clone(), &chord_payload.chord)?;
        }

        Ok(Some(chord_payload.chord.clone()))
    }

    fn should_execute_key_buffer_on_space_release(state: &ChorderState) -> bool {
        !state.key_buffer.is_empty() && state.active_chord.is_none()
    }

    // If an unshifted key is pressed, we append it to the key buffer, which always clears
    // our `active_chord`
    fn handle_unshifted_key_press(&self, handle: AppHandle, key: &Key) -> Result<()> {
        let state = self.state.get()?;
        let mut next_key_buffer = state.key_buffer.clone();
        next_key_buffer.push(key.clone());
        log::debug!("New key buffer: {:?}", next_key_buffer);
        self.state.set(Arc::new(ChorderState {
            key_buffer: next_key_buffer,
            pressed_chord: None,
            active_chord: None,
        }))?;
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
        self.state.set(Arc::new(ChorderState {
            // We always clear the key_buffer if a chord is pressed
            key_buffer: vec![],
            pressed_chord: Some(chord_payload.chord.clone()),
            active_chord: Some(chord_payload.chord.clone()),
        }))?;

        Ok(())
    }
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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

fn should_handle_held_key_event(held_keys: &mut HashSet<Key>, key_event: &KeyEvent) -> bool {
    match key_event {
        KeyEvent::Press(key) => held_keys.insert(*key),
        KeyEvent::Release(key) => {
            held_keys.remove(key);
            true
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn space_release_executes_only_when_buffered_chord_has_not_run_yet() {
        let buffered_state = ChorderState {
            key_buffer: vec![Key(KeyA)],
            pressed_chord: None,
            active_chord: None,
        };
        assert!(Chorder::should_execute_key_buffer_on_space_release(
            &buffered_state
        ));

        let executed_state = ChorderState {
            key_buffer: vec![Key(KeyA)],
            pressed_chord: Some(test_chord()),
            active_chord: Some(test_chord()),
        };
        assert!(!Chorder::should_execute_key_buffer_on_space_release(
            &executed_state
        ));

        let empty_state = ChorderState::new();
        assert!(!Chorder::should_execute_key_buffer_on_space_release(
            &empty_state
        ));
    }

    #[test]
    fn repeated_presses_are_ignored_until_release() {
        let mut held_keys = HashSet::new();
        let key = Key(KeyA);

        assert!(should_handle_held_key_event(
            &mut held_keys,
            &KeyEvent::Press(key)
        ));
        assert!(!should_handle_held_key_event(
            &mut held_keys,
            &KeyEvent::Press(key)
        ));
        assert!(should_handle_held_key_event(
            &mut held_keys,
            &KeyEvent::Release(key)
        ));
        assert!(should_handle_held_key_event(
            &mut held_keys,
            &KeyEvent::Press(key)
        ));
    }
}
