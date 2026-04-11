use crate::app::AppHandleExt;
use keycode::KeyMappingCode;
use std::os::raw::c_int;
use std::process::Command;
use std::sync::{OnceLock, mpsc::Sender};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::mpsc::channel;
use bitflags::bitflags;
use device_query::{DeviceQuery, DeviceState};
use tauri::AppHandle;
use crate::app::state::AppSingleton;
use anyhow::Result;
use crate::app::mode::AppModeManager;
use crate::models::{Key, KeyEvent, KeyEventAction};

static TX: OnceLock<Sender<bool>> = OnceLock::new();

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

pub struct AppKeyboard {
    pub device_state: Option<DeviceState>,
    pub modifier_flags: AtomicU16,

    pub(super) handle: AppHandle
}

impl AppKeyboard {
    pub fn register_input_handler(&self) -> Result<()> {
        let handle = self.handle.clone();
        let (tx, rx) = channel::<KeyEvent>();

        {
            let handle = self.handle.clone();
            // Spawning the handler in a separate thread to keep the key grabber callback as fast as possible
            std::thread::spawn(move || {
                while let Ok(event) = rx.recv() {
                    if let Err(e) = handle_key_event(handle.clone(), event) {
                        log::error!("Failed to handle key event: {e}");
                    }
                }
            });
        }

        std::thread::spawn(move || {
            let callback = move |event: rdev::Event| -> Option<rdev::Event> {
                // Synthetic, skip processing
                if event.source_user_data == 0xDEADBEEF || event.source_user_data == 0xDEADDEAD {
                    return Some(event);
                }

                if !handle
                    .state()
                    .dev_lockfile_detector()
                    .should_intercept_input_events()
                {
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

                let action = self.process_event(&key_event);

                if let Err(e) = tx.send(key_event) {
                    log::error!("Failed to send key event: {e}");
                }

                match action {
                    KeyEventAction::Consume => None,
                    _ => Some(event),
                }
            };

            if let Err(error) = rdev::grab(callback) {
                println!("Error: {:?}", error)
            }
        });

        Ok(())
    }

    pub fn register_caps_lock_input_handler(&self) -> Result<()> {
        log::info!("Registering caps lock handler");
        let (tx, rx) = channel();

        TX.set(tx)
            .map_err(|_| anyhow::anyhow!("failed to set tx"))?;

        std::thread::spawn(|| unsafe {
            start_caps_lock_listener(caps_lock_changed);
        });

        let handle = self.handle.clone();
        std::thread::spawn(move || {
            while let Ok(pressed) = rx.recv() {
                if pressed {
                    process_event(&KeyEvent::Press(Key(KeyMappingCode::CapsLock)));

                    if let Err(e) = handle_key_event(
                        handle.clone(),
                        KeyEvent::Press(Key(KeyMappingCode::CapsLock)),
                    ) {
                        log::error!("Failed to handle Caps Lock Press: {e}");
                    }
                } else {
                    process_event(&KeyEvent::Release(Key(KeyMappingCode::CapsLock)));

                    if let Err(e) = handle_key_event(
                        handle.clone(),
                        KeyEvent::Release(Key(KeyMappingCode::CapsLock)),
                    ) {
                        log::error!("Failed to handle Caps Lock Release: {e}");
                    }
                }
            }
        });

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

    fn process_event(&self, event: &KeyEvent) -> KeyEventAction {
        self.update_modifier_flags(&event);

        let Some(ref device_state) = self.device_state else {
            return KeyEventAction::Forward
        };

        let mode = self.handle.state().mode_manager().mode();
        // We consume all events in chord mode
        if mode.is_chord() {
            return KeyEventAction::Consume
        }
        // We only consume the space bar in idle mode if it's pressed while Caps is pressed
        else {
            let keys = device_state.get_keys();
            let is_caps_pressed = keys.iter().any(|&k| Key::from(k) == Key(KeyMappingCode::CapsLock));
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
    fn start_caps_lock_listener(cb: extern "C" fn(c_int));
    fn toggle_caps() -> c_int;
}

extern "C" fn caps_lock_changed(pressed: c_int) {
    log::debug!("caps_lock_changed: {}", pressed);
    if let Some(tx) = TX.get() {
        if let Err(e) = tx.send(pressed != 0) {
            log::error!("Failed to send caps lock changed event: {e}");
        }
    } else {
        log::error!("No tx found");
    }
}



pub fn handle_key_event(handle: AppHandle, key_event: KeyEvent) -> Result<()> {
    let app_mode = handle.state().mode();

    match app_mode {
        AppModeManager::Chord => {
            chorder.handle_key_event(&key_event)?;
        }
        AppModeManager::None => {
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
                // emit_caps_lock()?;
            }
        }
    }

    Ok(())
}