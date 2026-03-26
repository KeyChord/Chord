use self::ui::{ChorderIndicatorUi, NativeSurfaceRect};
use crate::app::chord_package::Chord;
use crate::app::chord_runner::runtime::ChordPayload;
use crate::app::{AppHandleExt, SafeAppHandle};
use crate::input::Key;
use crate::input::KeyEvent;
use crate::observables::{ChorderObservable, ChorderState, FrontmostObservable, Observable};
use anyhow::Result;
use device_query::{DeviceQuery, Keycode};
use keycode::KeyMappingCode;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{Emitter, Listener};

mod ui;

pub struct AppChorder {
    pub ui: ChorderIndicatorUi,
    held_keys: Mutex<HashSet<Key>>,

    observable: Arc<ChorderObservable>,
    handle: SafeAppHandle,
}

impl AppChorder {
    pub fn new(handle: SafeAppHandle, observable: Arc<ChorderObservable>) -> Result<Self> {
        let ui = ChorderIndicatorUi::new(handle.clone())?;
        let surface_window = ui.window.clone();
        let surface_handle = ui.handle.clone();
        let listener_window = surface_window.clone();
        listener_window.listen(
            "chorder-surface-rect",
            move |event| match serde_json::from_str::<NativeSurfaceRect>(event.payload()) {
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
            },
        );

        let preload_ui = ui.clone();
        ui.window.once("chorder-window-ready", move |_| {
            if let Err(error) = Self::preload_ui(&preload_ui) {
                log::error!("Failed to preload chorder panel: {error}");
            }
        });

        Ok(Self {
            ui,
            observable,
            handle,
            held_keys: Mutex::new(HashSet::new()),
        })
    }

    fn emit_will_show(window: &tauri::WebviewWindow) {
        if let Err(error) = window.emit("chorder-will-show", ()) {
            log::error!("Failed to emit chorder will-show event: {error}");
        }
    }

    fn emit_visibility_changed(&self, visible: bool) {
        if let Err(error) = self.ui.window.emit("chorder-visibility-changed", visible) {
            log::error!("Failed to emit chorder visibility change: {error}");
        }
    }

    fn prepare_surface_before_reveal(window: &tauri::WebviewWindow) {
        let (tx, rx) = mpsc::sync_channel(1);
        window.once("chorder-surface-ready", move |_| {
            let _ = tx.send(());
        });
        Self::emit_will_show(window);

        if rx.recv_timeout(Duration::from_millis(160)).is_err() {
            log::debug!("Timed out waiting for chorder surface to prepare before reveal");
        }
    }

    fn preload_ui(ui: &ChorderIndicatorUi) -> Result<()> {
        log::debug!("Preloading chorder panel");

        if ui.ensure_visible()? {
            Self::prepare_surface_before_reveal(&ui.window);
            ui.ensure_hidden()?;
        }

        Ok(())
    }

    pub fn ensure_active(&self) -> Result<()> {
        if self.ui.ensure_visible()? {
            Self::prepare_surface_before_reveal(&self.ui.window);
            self.ui.reveal()?;
            self.emit_visibility_changed(true);
        }
        Ok(())
    }

    pub fn ensure_inactive(&self) -> Result<()> {
        self.held_keys.lock().clear();
        let state = self.observable.get_state()?;
        if !self.ui.is_visible() && state.is_idle() {
            return Ok(());
        }

        self.observable.set_state(state.clear_session())?;
        if self.ui.ensure_hidden()? {
            self.emit_visibility_changed(false);
        }
        Ok(())
    }

    fn sync_shift_state(&self, is_shift_pressed: bool) -> Result<()> {
        let state = self.observable.get_state()?;
        if state.is_shift_pressed == is_shift_pressed {
            return Ok(());
        }

        self.observable
            .set_state(state.with_shift_pressed(is_shift_pressed))?;
        Ok(())
    }

    fn toggle_indicator_visibility(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        self.observable
            .set_state(state.toggled_indicator_visibility())?;
        Ok(())
    }

    // If `handle_key_event` is called, the state is guaranteed to be active
    pub fn handle_key_event(&self, key_event: &KeyEvent) -> Result<()> {
        if !self.track_held_key_event(key_event) {
            log::debug!("Ignoring repeated chord-mode key press: {:?}", key_event);
            return Ok(());
        }

        let (KeyEvent::Press(key) | KeyEvent::Release(key)) = key_event;
        if key == &Key(KeyMappingCode::ShiftLeft) || key == &Key(KeyMappingCode::ShiftRight) {
            self.sync_shift_state(matches!(key_event, KeyEvent::Press(_)))?;
            return Ok(());
        }

        if key == &Key(KeyMappingCode::Tab) {
            if matches!(key_event, KeyEvent::Press(_)) {
                self.toggle_indicator_visibility()?;
            }
            return Ok(());
        }

        // Don't handle any modifier key events
        let modifiers = Key::modifiers();
        if modifiers.contains(key) {
            log::debug!("Ignoring modifier key: {:?}", key);
            return Ok(());
        }

        let non_shift_modifiers = Key::non_shift_modifiers();
        let handle = self.handle.try_handle()?;
        let context = handle.app_context();
        let chord_runner = handle.chord_runner();
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
            .any(|key: Keycode| non_shift_modifiers.contains(&key.into()))
        {
            log::debug!(
                "Ignoring event because the following modifiers were held down: {:?}",
                device_keys
            );
            return Ok(());
        }

        match key_event {
            KeyEvent::Release(Key(code)) => {
                let state = self.observable.get_state()?;

                if let Some(pressed_chord) = &state.pressed_chord {
                    if code == &KeyMappingCode::CapsLock {
                        chord_runner.release_chord(pressed_chord)?;
                    } else if pressed_chord.keys.last().is_some_and(|k| &k.0 == code) {
                        chord_runner.release_chord(pressed_chord)?;
                    }
                }

                if code == &KeyMappingCode::Space {
                    if Self::should_execute_key_buffer_on_release(&state) {
                        let _ = self.execute_key_buffer(state.key_buffer.clone(), true)?;
                        self.observable.set_state(state.clear_session())?;
                    }
                }

                Ok(())
            }

            // If the caps lock key is pressed, it means we should execute (and clear) the chord
            // currently in `key_buffer`, or if empty, execute the last chord
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                self.ensure_active()?;

                let chord_registry = handle.app_chord_registry();
                let frontmost = handle.observable_state::<FrontmostObservable>()?;
                let state = self.observable.get_state()?;
                let key_buffer = state.key_buffer.clone();

                // An empty `key_buffer` means we should execute the last executed chord
                if key_buffer.is_empty() {
                    // If there isn't an active chord, then do nothing
                    let Some(last_chord) = &state.active_chord else {
                        log::error!("Key buffer is empty and no chord is active");
                        return Ok(());
                    };

                    let application_id = frontmost.frontmost_app_bundle_id.clone();
                    let chord_runtime =
                        chord_registry.get_chord_runtime(&last_chord.keys, application_id);
                    if let Some(chord_runtime) = chord_runtime {
                        chord_runner.press_chord(
                            chord_runtime,
                            &ChordPayload {
                                chord: last_chord.clone(),
                                num_times: 1,
                            },
                        )?;
                        self.observable.set_state(state.with_session(
                            vec![],
                            state.active_chord.clone(),
                            state.active_chord.clone(),
                        ))?;
                    } else {
                        // e.g. we ran it on a different app
                        log::error!("Last chord no longer applies");
                        self.observable.set_state(state.clear_session())?;
                    }

                    return Ok(());
                }

                // A non-empty key_buffer means we should execute the chord.
                log::debug!("Executing key_buffer {:?}", key_buffer);
                let Some(chord) = self.execute_key_buffer(key_buffer.clone(), false)? else {
                    self.observable.set_state(state.clear_session())?;
                    return Ok(());
                };

                self.observable.set_state(state.with_session(
                    vec![],
                    Some(chord.clone()),
                    Some(chord),
                ))?;
                Ok(())
            }
            KeyEvent::Press(key) => {
                // Ignore space presses
                if key == &Key(KeyMappingCode::Space) {
                    self.ensure_active()?;
                    return Ok(());
                }

                self.ensure_active()?;
                let is_shift_pressed = context.is_shift_pressed();
                if is_shift_pressed {
                    self.handle_shifted_key_press(key)
                } else {
                    self.handle_unshifted_key_press(key)
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
        key_buffer: Vec<Key>,
        release_immediately: bool,
    ) -> Result<Option<Chord>> {
        let handle = self.handle.try_handle()?;
        let chord_registry = handle.app_chord_registry();
        let frontmost = handle.observable_state::<FrontmostObservable>()?;
        let frontmost_application_id = frontmost.frontmost_app_bundle_id.clone();
        let Some(chord_runtime) =
            chord_registry.get_chord_runtime(&key_buffer, frontmost_application_id.clone())
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

        let chord_runner = handle.chord_runner();
        chord_runner.press_chord(chord_runtime, &chord_payload)?;

        if release_immediately {
            chord_runner.release_chord(&chord_payload.chord)?;
        }

        Ok(Some(chord_payload.chord.clone()))
    }

    fn should_execute_key_buffer_on_release(state: &ChorderState) -> bool {
        !state.key_buffer.is_empty() && state.active_chord.is_none()
    }

    fn next_key_buffer_for_unshifted_press(key_buffer: &[Key], key: &Key) -> Vec<Key> {
        let mut next_key_buffer = key_buffer.to_vec();
        if key == &Key(KeyMappingCode::Backspace) {
            next_key_buffer.pop();
        } else {
            next_key_buffer.push(*key);
        }
        next_key_buffer
    }

    // If an unshifted key is pressed, we update the key buffer, which always clears
    // our `active_chord`
    fn handle_unshifted_key_press(&self, key: &Key) -> Result<()> {
        let state = self.observable.get_state()?;
        let next_key_buffer = Self::next_key_buffer_for_unshifted_press(&state.key_buffer, key);
        log::debug!("New key buffer: {:?}", next_key_buffer);
        self.observable
            .set_state(state.with_session(next_key_buffer, None, None))?;
        Ok(())
    }

    // If shift is pressed, it means the user is trying to execute a chord.
    // If a chord is executed, we always reset `key_buffer`.
    fn handle_shifted_key_press(&self, key: &Key) -> Result<()> {
        let state = self.observable.get_state()?;
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

        let handle = self.handle.try_handle()?;
        let chord_runner = handle.chord_runner();
        let frontmost = handle.observable_state::<FrontmostObservable>()?;
        let chord_registry = handle.app_chord_registry();
        let frontmost_application_id = frontmost.frontmost_app_bundle_id.clone();
        let chord_runtime = chord_registry.get_chord_runtime(&sequence, frontmost_application_id);
        let (Some(chord_runtime), Some(chord_payload)) = (
            chord_runtime.clone(),
            chord_runtime.and_then(|r| r.get_chord(&sequence)),
        ) else {
            // We don't change the state for an invalid sequence
            log::debug!("Invalid sequence {:?}", sequence);
            return Ok(());
        };

        log::debug!("Pressing chord: {:?}", chord_payload);
        chord_runner.press_chord(chord_runtime, &chord_payload)?;
        self.observable.set_state(state.with_session(
            // We always clear the key_buffer if a chord is pressed
            vec![],
            Some(chord_payload.chord.clone()),
            Some(chord_payload.chord.clone()),
        ))?;

        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use KeyMappingCode::*;

    fn test_chord() -> Chord {
        Chord {
            keys: vec![Key(KeyA)],
            index: 0,
            name: "Test".to_string(),
            shortcut: None,
            shell: None,
            js: None,
        }
    }

    #[test]
    fn release_executes_only_when_buffered_chord_has_not_run_yet() {
        let buffered_state = ChorderState {
            key_buffer: vec![Key(KeyA)],
            pressed_chord: None,
            active_chord: None,
            is_shift_pressed: false,
            is_indicator_visible: true,
        };
        assert!(AppChorder::should_execute_key_buffer_on_release(
            &buffered_state
        ));

        let executed_state = ChorderState {
            key_buffer: vec![Key(KeyA)],
            pressed_chord: Some(test_chord()),
            active_chord: Some(test_chord()),
            is_shift_pressed: false,
            is_indicator_visible: true,
        };
        assert!(!AppChorder::should_execute_key_buffer_on_release(
            &executed_state
        ));

        let empty_state = ChorderState::default();
        assert!(!AppChorder::should_execute_key_buffer_on_release(
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

    #[test]
    fn unshifted_press_appends_non_backspace_keys() {
        let next = AppChorder::next_key_buffer_for_unshifted_press(&[Key(KeyA)], &Key(KeyB));

        assert_eq!(next, vec![Key(KeyA), Key(KeyB)]);
    }

    #[test]
    fn unshifted_backspace_removes_last_buffered_key() {
        let next = AppChorder::next_key_buffer_for_unshifted_press(
            &[Key(KeyA), Key(KeyB)],
            &Key(Backspace),
        );

        assert_eq!(next, vec![Key(KeyA)]);
    }

    #[test]
    fn unshifted_backspace_on_empty_buffer_is_noop() {
        let next = AppChorder::next_key_buffer_for_unshifted_press(&[], &Key(Backspace));

        assert!(next.is_empty());
    }
}
