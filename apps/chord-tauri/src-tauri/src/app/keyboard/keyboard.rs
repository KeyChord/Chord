use super::registration::Registration;
use crate::app::AppHandleExt;
use crate::app::state::AppSingleton;
use crate::models::{AppKeyboardState, Key, KeyEvent, KeyEventAction};
use crate::state::KeyboardObservable;
use anyhow::Result;
use bitflags::bitflags;
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use nject::injectable;
use rdev::Key::KeyE;
use std::os::raw::c_int;
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use tauri::{AppHandle, Manager};

const CAPS_LOCK_QUEUE_CAPACITY: usize = 16;
const KEY_EVENT_QUEUE_CAPACITY: usize = 1_024;

static CAPS_LOCK_QUEUE_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);
static KEY_EVENT_QUEUE_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);
static INPUT_REGISTRATION: Registration = Registration::new();
static CAPS_REGISTRATION: Registration = Registration::new();
static CAPS_RUNNING: AtomicBool = AtomicBool::new(false);
static TX: OnceLock<(SyncSender<(u64, bool)>, AppHandle)> = OnceLock::new();

bitflags! {
  pub struct Modifiers: u16 {
      const LEFT_SHIFT      = 1 << 0;
      const RIGHT_SHIFT     = 1 << 1;
      const LEFT_CONTROL    = 1 << 2;
      const RIGHT_CONTROL   = 1 << 3;
      const LEFT_OPTION     = 1 << 4;
      const RIGHT_OPTION    = 1 << 5;
      const LEFT_COMMAND    = 1 << 6;
      const RIGHT_COMMAND   = 1 << 7;
      const FUNCTION        = 1 << 8;
  }
}

#[injectable]
pub struct AppKeyboard {
    keyboard_state: AppKeyboardState,

    #[inject(AtomicU16::new(0))]
    pub modifier_flags: AtomicU16,

    handle: AppHandle,
}

impl AppKeyboard {
    pub fn state(&self) -> &AppKeyboardState {
        &self.keyboard_state
    }

    pub fn input_handlers_running(&self) -> bool {
        INPUT_REGISTRATION.is_registered() && CAPS_RUNNING.load(Ordering::Acquire)
    }

    pub fn reset_input_state(&self) {
        self.keyboard_state.reset();
        self.modifier_flags.store(0, Ordering::Relaxed);
    }

    pub fn register_input_handler(&self) -> Result<()> {
        let Some(registration) = INPUT_REGISTRATION.acquire() else {
            return Ok(());
        };
        let handle = self.handle.clone();
        let (tx, rx) = sync_channel::<(u64, KeyEvent)>(KEY_EVENT_QUEUE_CAPACITY);

        {
            let handle = self.handle.clone();
            // Spawning the handler in a separate thread to keep the key grabber callback as fast as possible
            std::thread::Builder::new()
                .name("chord-key-events".into())
                .spawn(move || {
                    while let Ok((generation, event)) = rx.recv() {
                        handle
                            .app_state()
                            .dev_lockfile_detector()
                            .with_input_generation(generation, || {
                                if let Err(e) =
                                    handle.app_state().app_controller().handle_key_event(&event)
                                {
                                    log::error!("Failed to handle key event: {e}");
                                }
                            });
                    }
                })?;
        }

        std::thread::Builder::new().name("chord-key-tap".into()).spawn(move || {
            let _registration = registration;
            let callback = move |event: rdev::Event| -> Option<rdev::Event> {
                // Synthetic, skip processing
                if event.source_user_data == 0xDEADBEEF || event.source_user_data == 0xDEADDEAD {
                    return Some(event);
                }

                let key_event = match event.event_type {
                    rdev::EventType::KeyPress(key) => {
                        let Ok(key) = Key::try_from(key) else {
                            return Some(event);
                        };
                        KeyEvent::Press(key)
                    }
                    rdev::EventType::KeyRelease(key) => {
                        let Ok(key) = Key::try_from(key) else {
                            return Some(event);
                        };
                        KeyEvent::Release(key)
                    }
                    _ => return Some(event),
                };

                let action = handle.app_state().dev_lockfile_detector().with_input_owner(|generation| {
                    let action = handle.app_state().keyboard().handle_key_event(&key_event);
                    match tx.try_send((generation, key_event)) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_)) => {
                        if !KEY_EVENT_QUEUE_WARNING_EMITTED.swap(true, Ordering::Relaxed) {
                            log::warn!(
                                "Dropping keyboard events because the bounded input queue is saturated"
                            );
                        }
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        log::error!("Keyboard event handler is unavailable");
                    }
                }

                    action
                }).unwrap_or(KeyEventAction::Forward);
                match action {
                    KeyEventAction::Consume => None,
                    _ => Some(event),
                }
            };

            if let Err(error) = rdev::grab(callback) {
                log::error!("Keyboard event tap stopped: {error:?}");
            }
        })?;

        Ok(())
    }

    pub fn register_caps_lock_input_handler(&self) -> Result<()> {
        let Some(registration) = CAPS_REGISTRATION.acquire() else {
            return Ok(());
        };
        log::info!("Registering caps lock handler");
        // One receiver for the lifetime of the process, including native startup retries.
        if TX.get().is_none() {
            let (tx, rx) = sync_channel::<(u64, bool)>(CAPS_LOCK_QUEUE_CAPACITY);
            let handle = self.handle.clone();
            std::thread::Builder::new()
                .name("chord-caps-events".into())
                .spawn(move || {
                    while let Ok((generation, pressed)) = rx.recv() {
                        handle
                            .app_state()
                            .dev_lockfile_detector()
                            .with_input_generation(generation, || {
                                let key = Key(KeyMappingCode::CapsLock);
                                let event = if pressed {
                                    KeyEvent::Press(key)
                                } else {
                                    KeyEvent::Release(key)
                                };
                                handle.app_state().keyboard().handle_key_event(&event);
                                if let Err(e) =
                                    handle.app_state().app_controller().handle_key_event(&event)
                                {
                                    log::error!("Failed to handle Caps Lock event: {e}");
                                }
                            });
                    }
                })?;
            TX.set((tx, self.handle.clone()))
                .map_err(|_| anyhow::anyhow!("caps channel already initialized"))?;
        }
        std::thread::Builder::new()
            .name("chord-caps-hid".into())
            .spawn(move || {
                let _registration = registration;
                let result = unsafe { start_caps_lock_listener(caps_lock_changed) };
                CAPS_RUNNING.store(false, Ordering::Release);
                log::error!("Caps Lock HID listener stopped: {result}");
            })?;
        Ok(())
    }

    pub fn emit_caps_lock() -> Result<()> {
        let rc = unsafe { toggle_caps() };
        if rc == 0 {
            Ok(())
        } else {
            anyhow::bail!("failed to toggle caps lock state via native layer: {rc}")
        }
    }

    pub fn set_caps_lock_off() -> Result<()> {
        let rc = unsafe { set_caps_off() };
        if rc == 0 {
            Ok(())
        } else {
            anyhow::bail!("failed to toggle caps lock state via native layer: {rc}")
        }
    }

    fn handle_key_event(&self, event: &KeyEvent) -> KeyEventAction {
        log::debug!("Processing event {event:?}");
        self.keyboard_state.handle_key_event(event);

        self.update_modifier_flags(&event);

        let app_mode = self.handle.app_state().app_controller().app_mode();

        // We only consume the space bar in idle mode if it's pressed while Caps is pressed
        // TODO: Only disable if Chord mode was activated. Naive approach doesn't work.
        if event == &KeyEvent::Press(Key(KeyMappingCode::CapsLock)) {
            log::debug!("Ensuring Caps is off");
            Self::set_caps_lock_off();
        }

        if event == &KeyEvent::Press(Key(KeyMappingCode::Space))
            && self.keyboard_state.is_caps_pressed()
        {
            return KeyEventAction::Consume;
        }

        if app_mode.is_chord() {
            return KeyEventAction::Consume;
        }

        KeyEventAction::Forward
    }

    #[allow(dead_code)]
    pub fn get_modifier_flags(&self) -> Modifiers {
        Modifiers::from_bits_truncate(self.modifier_flags.load(Ordering::Relaxed))
    }

    fn modifier_key_to_flag(key: &Key) -> Option<Modifiers> {
        let flag = match key.0 {
            KeyMappingCode::ShiftLeft => Modifiers::LEFT_SHIFT,
            KeyMappingCode::ShiftRight => Modifiers::RIGHT_SHIFT,
            KeyMappingCode::ControlLeft => Modifiers::LEFT_CONTROL,
            KeyMappingCode::ControlRight => Modifiers::RIGHT_CONTROL,
            KeyMappingCode::AltLeft => Modifiers::LEFT_OPTION,
            KeyMappingCode::AltRight => Modifiers::RIGHT_OPTION,
            KeyMappingCode::MetaLeft => Modifiers::LEFT_COMMAND,
            KeyMappingCode::MetaRight => Modifiers::RIGHT_COMMAND,
            KeyMappingCode::Fn => Modifiers::FUNCTION,
            _ => return None,
        };

        Some(flag)
    }

    fn update_modifier_flags(&self, event: &KeyEvent) {
        match event {
            KeyEvent::Press(key) => {
                if let Some(flag) = Self::modifier_key_to_flag(key) {
                    self.modifier_flags.fetch_or(flag.bits(), Ordering::Relaxed);
                }
            }
            KeyEvent::Release(key) => {
                if let Some(flag) = Self::modifier_key_to_flag(key) {
                    self.modifier_flags
                        .fetch_and(!flag.bits(), Ordering::Relaxed);
                }
            }
        }
    }
}

unsafe extern "C" {
    fn start_caps_lock_listener(cb: extern "C" fn(c_int)) -> c_int;
    fn toggle_caps() -> c_int;
    fn set_caps_off() -> c_int;
}

extern "C" fn caps_lock_changed(pressed: c_int) {
    // Native device availability: only claim ownership with an open keyboard.
    if pressed == -1 || pressed == -2 {
        CAPS_RUNNING.store(pressed == -1, Ordering::Release);
        return;
    }
    if let Some((tx, handle)) = TX.get() {
        handle
            .app_state()
            .dev_lockfile_detector()
            .with_input_owner(|generation| match tx.try_send((generation, pressed != 0)) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => {
                    if !CAPS_LOCK_QUEUE_WARNING_EMITTED.swap(true, Ordering::Relaxed) {
                        log::warn!(
                            "Dropping caps-lock events because the bounded input queue is saturated"
                        );
                    }
                }
                Err(TrySendError::Disconnected(_)) => {
                    CAPS_RUNNING.store(false, Ordering::Release);
                    log::error!("Caps-lock event handler is unavailable");
                }
            });
    }
}
