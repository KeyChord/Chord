use self::ui::{ChorderIndicatorUi, NativeSurfaceRect};
use crate::app::{AppHandleExt};
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
use tauri::{AppHandle, Emitter, Listener};
use crate::app::chord_runner::{ChordActionTask, ChordActionTaskRun};
use crate::app::state::StateSingleton;
use crate::models::ChordInput;

mod ui;

pub struct AppChorder {
    pub ui: ChorderIndicatorUi,
    active_task_run: Mutex<Option<ChordActionTaskRun>>,

    /// Prevents double-pressing keys when a key is held for a while
    held_keys: Mutex<HashSet<Key>>,

    observable: ChorderObservable,
    handle: AppHandle,
}

impl StateSingleton for AppChorder {
    fn new(handle: AppHandle) -> Self {
        Self {
            ui: ChorderIndicatorUi::new(handle.clone()),
            active_task_run: Mutex::new(None),
            held_keys: Mutex::new(HashSet::new()),
            observable: ChorderObservable::placeholder(),
            handle,
        }
    }
}

impl AppChorder {
    pub fn init(&self, observable: ChorderObservable) -> Result<()> {
        self.observable.init(observable);

        self.ui.init()?;
        let surface_window = self.ui.get_or_create_window()?;
        let surface_handle = self.ui.handle.clone();
        let listener_window = surface_window.clone();
        let listener_window2 = surface_window.clone();
        listener_window.listen(
            "chorder-surface-rect",
            move |event| match serde_json::from_str::<NativeSurfaceRect>(event.payload()) {
                Ok(rect) => {
                    if let Err(error) = ChorderIndicatorUi::configure_window_surface(
                        &listener_window2,
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

        Ok(())
    }

    fn emit_will_show(&self) -> Result<()> {
        let window = self.ui.get_or_create_window()?;
        window.emit("chorder-will-show", ())?;
        Ok(())
    }

    fn emit_visibility_changed(&self, visible: bool) -> Result<()> {
        let window = self.ui.get_or_create_window()?;
        window.emit("chorder-visibility-changed", visible)?;
        Ok(())
    }

    fn prepare_surface_before_reveal(&self) -> Result<()> {
        let window = self.ui.get_or_create_window()?;
        let (tx, rx) = mpsc::sync_channel(1);
        window.once("chorder-surface-ready", move |_| {
            let _ = tx.send(());
        });
        self.emit_will_show()?;
        rx.recv_timeout(Duration::from_millis(160))?;
        Ok(())
    }

    fn preload_ui(&self, ui: &ChorderIndicatorUi) -> Result<()> {
        log::debug!("Preloading chorder panel");

        if ui.ensure_visible()? {
            self.prepare_surface_before_reveal()?;
            ui.ensure_hidden()?;
        }

        Ok(())
    }

    pub fn ensure_active(&self) -> Result<()> {
        if self.ui.ensure_visible()? {
            self.prepare_surface_before_reveal();
            self.ui.reveal()?;
            self.emit_visibility_changed(true)?;
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
            self.emit_visibility_changed(false)?;
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

    fn spawn_end_task(&self, task_run: ChordActionTaskRun) -> Result<()> {
        let handle = self.handle.clone();
        tauri::async_runtime::spawn(async move {
            let chord_action_task_runner = handle.chord_action_task_runner();
            if let Err(e) = chord_action_task_runner.end_task(task_run).await {
                log::error!("Error: {}", e);
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
            self.sync_shift_state(matches!(key_event, KeyEvent::Press(_)))?;
            return Ok(());
        }

        let state = self.observable.get_state()?;
        if Self::should_toggle_indicator_for_tab_event(key_event, state.is_shift_pressed) {
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
        let context = self.handle.app_context();
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

        Ok(match key_event {
            KeyEvent::Release(Key(code)) => {
                let state = self.observable.get_state()?;

                if let Some(pressed_chord_keys) = &state.pressed_chord_keys {
                    if code == &KeyMappingCode::CapsLock  || pressed_chord_keys.last().is_some_and(|k| &k.0 == code) {
                        self.spawn_end_active_task()?;
                    }
                }

                if code == &KeyMappingCode::Space {
                    if Self::should_execute_key_buffer_on_release(&state) {
                        if let Some(task) = self.resolve_task_from_keys(&state.key_buffer, 1)? {
                            self.run_task(task)?;
                            self.spawn_end_active_task()?;
                        };
                        self.observable.set_state(state.clear_session())?;
                    }
                }
            }

            // If the caps lock key is pressed, it means we should execute (and clear) the chord
            // currently in `key_buffer`, or if empty, execute the last chord
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                self.ensure_active()?;
                let state = self.observable.get_state()?;
                let key_buffer = state.key_buffer.clone();

                // An empty `key_buffer` means we should execute the last executed chord
                if key_buffer.is_empty() {
                    // If there isn't an active chord, then do nothing
                    let Some(active_chord_keys) = &state.active_chord_keys else {
                        log::error!("Key buffer is empty and no chord is active");
                        return Ok(());
                    };
                    if let Some(task) = self.resolve_task_from_keys(active_chord_keys, 1)? {
                        self.run_task(task)?;
                        self.observable.set_state(state.with_session(
                            vec![],
                            state.active_chord_keys.clone(),
                            state.active_chord_keys.clone(),
                        ))?;
                    } else {
                        // e.g. we ran it on a different app
                        log::error!("Last chord no longer applies");
                        self.observable.set_state(state.clear_session())?;
                    }
                } else {
                    // A non-empty key_buffer means we should execute the chord.
                    log::debug!("Executing key_buffer {:?}", key_buffer);
                    let Some(task) = self.resolve_task_from_keys(&key_buffer, 1)? else {
                        self.observable.set_state(state.clear_session())?;
                        return Ok(());
                    };

                    self.run_task(task)?;
                    self.observable.set_state(state.with_session(
                        vec![],
                        Some(key_buffer.clone()),
                        Some(key_buffer),
                    ))?;
                }
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

    fn resolve_task_from_keys(&self, keys: &[Key], num_times: u32) -> Result<Option<ChordActionTask>> {
        let frontmost = self.handle.observable_state::<FrontmostObservable>()?;
        let application_id = frontmost.frontmost_app_bundle_id.clone();
        let chord_package_manager = self.handle.chord_package_manager();
        let input = ChordInput { keys: keys.to_vec(), application_id };
        let Some(chord_package) = chord_package_manager.resolve_package_for_input(&input) else {
            return Ok(None);
        };
        let Some(chord) = chord_package.resolve_chord_for_input(&input) else {
            return Ok(None);
        };
        let Some(action) = chord.actions.first() else {
            return Ok(None)
        };
        let task = ChordActionTask {
            action: action.clone(),
            num_times
        };
        Ok(Some(task))
    }

    fn spawn_end_active_task(&self) -> Result<()>{
        let mut guard = self.active_task_run.lock();
        if let Some(active_task_run) = std::mem::take(&mut *guard) {
            self.spawn_end_task(active_task_run)?;
        }
        Ok(())
    }

    fn run_task(&self, task: ChordActionTask) -> Result<()> {
        let runner = self.handle.chord_action_task_runner();
        let task_run = runner.start_task(&task)?;
        let mut lock = self.active_task_run.lock();
        *lock = Some(task_run);
        Ok(())
    }

    fn should_execute_key_buffer_on_release(state: &ChorderState) -> bool {
        !state.key_buffer.is_empty() && state.active_chord_keys.is_none()
    }

    fn should_toggle_indicator_for_tab_event(key_event: &KeyEvent, is_shift_pressed: bool) -> bool {
        matches!(key_event, KeyEvent::Press(Key(KeyMappingCode::Tab)) | KeyEvent::Release(Key(KeyMappingCode::Tab)))
            && is_shift_pressed
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

    fn should_clear_key_buffer_on_shifted_press(key: &Key) -> bool {
        key == &Key(KeyMappingCode::Backspace)
    }

    fn handle_shift_backspace_press(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        self.observable
            .set_state(state.with_session(vec![], None, None))?;
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
                let Some(active_chord_keys) = &state.active_chord_keys else {
                    // If `key_buffer` and `active_chord` is empty, then we do nothing
                    log::error!("No chord active");
                    return Ok(());
                };

                let mut new_chord = active_chord_keys.clone();
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

        self.run_task(task)?;
        self.observable.set_state(state.with_session(
            // We always clear the key_buffer if a chord is pressed
            vec![],
            Some(sequence.clone()),
            Some(sequence.clone()),
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
