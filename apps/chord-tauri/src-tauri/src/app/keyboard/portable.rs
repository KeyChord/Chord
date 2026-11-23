use super::caps_lock_guard::CapsLockGuard;
use super::health::{Backoff, InputHealth};
use crate::app::AppHandleExt;
use crate::desktop_platform as platform;
use crate::models::{AppKeyboardState, CapturedKeyEvent, Key, KeyEvent};
use anyhow::Result;
use keycode::KeyMappingCode::CapsLock;
use nject::injectable;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::time::{Duration, Instant};
use tauri::AppHandle;

const SUPERVISOR_INTERVAL: Duration = Duration::from_millis(250);

#[injectable]
pub struct AppKeyboard {
    keyboard_state: AppKeyboardState,
    handle: AppHandle,
    #[inject(InputHealth::default())]
    health: InputHealth,
    #[inject(CapsLockGuard::default())]
    caps: CapsLockGuard,
    #[inject(AtomicBool::new(false))]
    capture: AtomicBool,
    #[inject(Mutex::new(HashSet::new()))]
    consumed: Mutex<HashSet<Key>>,
}

impl AppKeyboard {
    pub fn state(&self) -> &AppKeyboardState {
        &self.keyboard_state
    }

    pub fn input_handlers_running(&self) -> bool {
        self.health.tap_running()
    }

    pub fn reset_input_state(&self) {
        self.on_chord_mode_ended();
        self.keyboard_state.reset();
    }

    pub fn set_input_owner(&self, _active: bool) {}

    pub fn release_input(&self) {
        self.reset_input_state();
    }

    pub fn on_normal_mode_started(&self) {
        self.caps
            .normal_mode_started(Self::get_caps_lock().unwrap_or(false));
        Self::apply_caps(false);
    }

    pub fn on_chord_mode_ended(&self) {
        self.capture.store(false, Ordering::Release);
        if let Some(enabled) = self.caps.chord_ended() {
            Self::apply_caps(enabled);
        }
    }

    pub fn held_modifier_keys() -> Vec<Key> {
        platform::held_modifier_keys()
    }

    pub fn get_caps_lock() -> Result<bool> {
        platform::caps_lock()
    }

    fn apply_caps(enabled: bool) {
        if let Err(error) = platform::set_caps_lock(enabled) {
            log::warn!("Could not change Caps Lock: {error:#}");
        }
    }

    pub(super) fn start_supervisor(&self) -> Result<()> {
        let (send, receive) = sync_channel::<(u64, CapturedKeyEvent)>(1024);
        let handle = self.handle.clone();
        std::thread::Builder::new()
            .name("chord-key-events".into())
            .spawn(move || {
                while let Ok((generation, event)) = receive.recv() {
                    let state = handle.app_state();
                    state
                        .dev_lockfile_detector()
                        .with_input_generation(generation, || {
                            if event.activates_chord_mode() {
                                state.frontmost().capture_current();
                            }
                            state.keyboard().handle_caps(&event.event);
                            if let Err(error) = state.app_controller().handle_key_event(&event) {
                                log::error!("Failed to handle key event: {error:#}");
                            }
                        });
                }
            })?;
        let handle = self.handle.clone();
        std::thread::Builder::new()
            .name("chord-input".into())
            .spawn(move || {
                let mut backoff = Backoff::default();
                let stop = AtomicBool::new(false);
                loop {
                    let state = handle.app_state();
                    let keyboard = state.keyboard();
                    if state
                        .permissions()
                        .refresh_if_changed()
                        .is_ok_and(|p| p.accessibility)
                        && backoff.ready(Instant::now())
                    {
                        let filter_handle = handle.clone();
                        let send = send.clone();
                        let ready_handle = handle.clone();
                        let ready = move || {
                            ready_handle
                                .app_state()
                                .keyboard()
                                .health
                                .set_tap_running(true)
                        };
                        let owner_handle = handle.clone();
                        let result = platform::listen(
                            &stop,
                            Box::new(move |event| {
                                filter_handle.app_state().keyboard().filter(&send, event)
                            }),
                            Box::new(ready),
                            move || {
                                owner_handle
                                    .app_state()
                                    .dev_lockfile_detector()
                                    .with_input_owner(|_| ())
                                    .is_some()
                            },
                        );
                        keyboard.health.set_tap_running(false);
                        keyboard.consumed.lock().clear();
                        let delay = backoff.failed(Instant::now());
                        if let Err(error) = result {
                            log::warn!(
                                "Keyboard listener stopped (retrying in {delay:?}): {error:#}"
                            );
                        }
                    }
                    std::thread::sleep(SUPERVISOR_INTERVAL);
                }
            })?;
        Ok(())
    }

    fn filter(&self, send: &SyncSender<(u64, CapturedKeyEvent)>, event: KeyEvent) -> bool {
        let released = if let KeyEvent::Release(key) = event {
            Some(key)
        } else {
            None
        };
        let state = self.handle.app_state();
        state
            .dev_lockfile_detector()
            .with_input_owner(|generation| {
                let input = CapturedKeyEvent {
                    event,
                    caps_armed: self.keyboard_state.is_caps_armed(),
                };
                let capture = self.capture.load(Ordering::Acquire);
                let mut consumed = self.consumed.lock();
                let take = match input.event {
                    KeyEvent::Press(key) => {
                        if key.0 == CapsLock && self.keyboard_state.is_caps_pressed() {
                            return true;
                        }
                        if key.0 == CapsLock {
                            self.keyboard_state
                                .arm_caps(Self::get_caps_lock().unwrap_or(false));
                        }
                        if input.activates_chord_mode() {
                            self.capture.store(true, Ordering::Release);
                        }
                        let take = key.0 == CapsLock || capture || input.activates_chord_mode();
                        if take {
                            consumed.insert(key);
                        }
                        take
                    }
                    KeyEvent::Release(key) => consumed.remove(&key),
                };
                self.keyboard_state.handle_key_event(&input.event);
                // Release of a forwarded key must reach the original app, even during a chord.
                if capture || take {
                    if send.try_send((generation, input)).is_err() {
                        log::warn!("Chord's input queue is full or unavailable");
                        self.capture.store(false, Ordering::Release);
                    }
                }
                take
            })
            .unwrap_or_else(|| released.is_some_and(|key| self.consumed.lock().remove(&key)))
    }

    fn handle_caps(&self, event: &KeyEvent) {
        let enabled = match event {
            KeyEvent::Press(Key(CapsLock)) => {
                let state = self.handle.app_state();
                let normal = state.app_controller().app_mode().is_chord()
                    && !state.chord_mode_manager().is_inserting();
                self.caps
                    .pressed(Self::get_caps_lock().unwrap_or(false), normal)
            }
            KeyEvent::Release(Key(CapsLock)) => self.caps.released(),
            _ => None,
        };
        if let Some(enabled) = enabled {
            Self::apply_caps(enabled);
        }
    }
}
