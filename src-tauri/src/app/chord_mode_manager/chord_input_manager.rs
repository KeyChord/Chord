use crate::app::AppHandleExt;
use crate::app::chord_runner::{ChordActionTask, ChordActionTaskRun};
use crate::app::state::AppSingleton;
use crate::models::{ChordInput, ChordInputEvent, Key, KeyEvent};
use crate::state::{ChordInputObservable, ChordInputState, ChordModeState, FrontmostObservable, Observable};
use anyhow::Result;
use device_query::{DeviceQuery, Keycode};
use keycode::KeyMappingCode;
use nject::injectable;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener};
use tauri_plugin_user_input::InputEvent;

#[injectable]
pub struct ChordInputManager {
    #[inject(Mutex::new(None))]
    active_task_run: Mutex<Option<ChordActionTaskRun>>,

    /// Prevents double-pressing keys when a key is held for a while
    #[inject(Mutex::new(HashSet::new()))]
    held_keys: Mutex<HashSet<Key>>,

    handle: AppHandle,
    observable: ChordInputObservable,
}

impl ChordInputManager {
    pub fn reset(&self) -> Result<()> {
        self.observable.set_state(|_| ChordInputState::default())
    }

    fn spawn_end_task(&self, task_run: ChordActionTaskRun) -> Result<()> {
        let handle = self.handle.clone();
        tauri::async_runtime::spawn(async move {
            let chord_action_task_runner = handle.app_state().chord_action_task_runner();
            if let Err(e) = chord_action_task_runner.end_task(task_run).await {
                log::error!("error ending task: {:?}", e);
            }
        });
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
            return Ok(());
        }

        let is_shift_pressed = self.observable.get_state()?.is_shift_pressed;
        if Self::should_toggle_indicator_for_tab_event(key_event, is_shift_pressed) {
            if matches!(key_event, KeyEvent::Press(_)) {
                let chord_mode_manager = self.handle.app_state().chord_mode_manager();
                chord_mode_manager.toggle_indicator_visibility();
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
        let keyboard = self.handle.app_state().keyboard();
        let Some(device_state) = &keyboard.device_state else {
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

        Ok(match key_event {
            KeyEvent::Release(Key(code)) => {
                let state = self.observable.get_state()?;

                if let Some(pressed_input_event) = &state.pressed_input_event {
                    if code == &KeyMappingCode::CapsLock
                        || pressed_input_event.input.last().is_some_and(|k| &k.0 == code)
                    {
                        self.spawn_end_active_task()?;
                    }
                }

                if code == &KeyMappingCode::Space {
                    if Self::should_execute_key_buffer_on_release(&state) {
                        if let Some(task) = self.resolve_task_from_keys(&state.input, 1)? {
                            self.run_task(task)?;
                            self.spawn_end_active_task()?;
                        };
                        self.observable.set_state(|_| ChordInputState::default())?;
                    }
                }
            }

            // If the caps lock key is pressed, it means we should execute (and clear) the chord
            // currently in `key_buffer`, or if empty, execute the last chord
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                let state = self.observable.get_state()?;
                let key_buffer = state.input.clone();

                // An empty `key_buffer` means we should execute the last executed chord
                if key_buffer.is_empty() {
                    // If there isn't an active chord, then do nothing
                    let Some(selected_input_event) = &state.selected_input_event else {
                        log::debug!("Key buffer is empty and no chord is active");
                        return Ok(());
                    };
                    if let Some(task) = self.resolve_task_from_keys(&selected_input_event.input, 1)? {
                        self.run_task(task)?;
                        self.observable.set_state(|state| {
                            ChordInputState {
                                input: vec![],
                                pressed_input_event: state.selected_input_event.clone(),
                                selected_input_event: state.selected_input_event,
                                ..state
                            }
                        })?;
                    } else {
                        // e.g. we ran it on a different app
                        log::error!("Last chord no longer applies");
                        self.observable.set_state(|_| ChordInputState::default())?;
                    }
                } else {
                    // A non-empty key_buffer means we should execute the chord.
                    log::debug!("Executing key_buffer {:?}", key_buffer);
                    let Some(task) = self.resolve_task_from_keys(&key_buffer, 1)? else {
                        log::debug!("task not found for key_buffer {:?}", key_buffer);
                        self.observable.set_state(|_| ChordInputState::default())?;
                        return Ok(());
                    };
                    let event = task.event.clone();
                    self.observable.set_state(move |prev| {
                        ChordInputState {
                            input: vec![],
                            pressed_input_event: Some(event.clone()),
                            selected_input_event: Some(event),
                            ..prev
                        }
                    })?;

                    self.run_task(task)?;
                }
            }
            KeyEvent::Press(key) => {
                // Ignore space presses
                if key == &Key(KeyMappingCode::Space) {
                    return Ok(());
                }

                let is_shift_pressed = self.observable.get_state()?.is_shift_pressed;
                if is_shift_pressed {
                    if Self::should_clear_key_buffer_on_shifted_press(key) {
                        self.handle_shift_backspace_press()?;
                    } else {
                        self.handle_shifted_key_press(key)?;
                    }
                } else {
                    self.handle_unshifted_key_press(key)?;
                }
            }
        })
    }

    fn track_held_key_event(&self, key_event: &KeyEvent) -> bool {
        let mut held_keys = self.held_keys.lock();
        should_handle_held_key_event(&mut held_keys, key_event)
    }

    fn resolve_task_from_keys(
        &self,
        keys: &[Key],
        num_times: u32,
    ) -> Result<Option<ChordActionTask>> {
        let frontmost = self.handle.app_state().frontmost();
        let application_id = frontmost.frontmost()?;
        let chord_package_manager = self.handle.app_state().chord_package_manager();
        let event = ChordInputEvent {
            input: keys.to_vec(),
            application_id,
        };
        let Some(chord_package) = chord_package_manager.create_event_context(&event) else {
            log::debug!("package for input {:?} not found", event);
            return Ok(None);
        };
        log::debug!("found package {} for input {:?}", chord_package.name, event);
        let Some(chord_ref) = chord_package.resolve_chord_from_input_event(&event) else {
            log::debug!(
                "couldn't resolve chord in package {} for input {:?}",
                chord_package.name,
                event
            );
            return Ok(None);
        };
        log::debug!("resolved chord: {:?}", chord_ref);
        let task = chord_package.resolve_task(event, chord_ref, num_times)?;
        log::debug!("resolved task: {:?}", task);
        Ok(task)
    }

    fn spawn_end_active_task(&self) -> Result<()> {
        let mut guard = self.active_task_run.lock();
        if let Some(active_task_run) = std::mem::take(&mut *guard) {
            self.spawn_end_task(active_task_run)?;
        }
        Ok(())
    }

    fn run_task(&self, task: ChordActionTask) -> Result<()> {
        let runner = self.handle.app_state().chord_action_task_runner();
        let task_run = runner.start_task(&task)?;
        let mut lock = self.active_task_run.lock();
        *lock = Some(task_run);
        Ok(())
    }

    fn should_execute_key_buffer_on_release(state: &ChordInputState) -> bool {
        !state.input.is_empty() && state.selected_input_event.is_none()
    }

    fn should_toggle_indicator_for_tab_event(key_event: &KeyEvent, is_shift_pressed: bool) -> bool {
        matches!(
            key_event,
            KeyEvent::Press(Key(KeyMappingCode::Tab)) | KeyEvent::Release(Key(KeyMappingCode::Tab))
        ) && is_shift_pressed
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
        let next_key_buffer = Self::next_key_buffer_for_unshifted_press(&state.input, key);
        log::debug!("New key buffer: {:?}", next_key_buffer);
        self.observable
            .set_state(|state| ChordInputState { input: next_key_buffer, selected_input_event: None, pressed_input_event: None, ..state})?;
        Ok(())
    }

    fn should_clear_key_buffer_on_shifted_press(key: &Key) -> bool {
        key == &Key(KeyMappingCode::Backspace)
    }

    fn handle_shift_backspace_press(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        self.observable
            .set_state(|state| ChordInputState { input: vec![], selected_input_event: None, pressed_input_event: None, ..state})?;
        Ok(())
    }

    // If shift is pressed, it means the user is trying to execute a chord.
    // If a chord is executed, we always reset `key_buffer`.
    fn handle_shifted_key_press(&self, key: &Key) -> Result<()> {
        let state = self.observable.get_state()?;
        let key_buffer = state.input.clone();

        let sequence = {
            // If key_buffer is empty (i.e. we just activated a chord), we should use that chord to
            // determine our sequence
            if key_buffer.is_empty() {
                let Some(selected_input_event) = &state.selected_input_event else {
                    // If `key_buffer` and `active_chord` is empty, then we do nothing
                    log::error!("No chord active");
                    return Ok(());
                };

                let mut new_chord = selected_input_event.input.clone();
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

        let Some(task) = self.resolve_task_from_keys(&sequence, 1)? else {
            log::debug!("Invalid sequence {:?}", sequence);
            return Ok(());
        };

        let event = task.event.clone();
        self.observable.set_state(move |state| ChordInputState {
            input: vec![],
            pressed_input_event: Some(event.clone()),
            selected_input_event: Some(event),
            ..state
        });
        self.run_task(task)?;

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
