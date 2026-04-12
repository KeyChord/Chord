use std::sync::atomic::{AtomicBool, Ordering};
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use crate::models::{Key, KeyEvent};

pub struct AppKeyboardState {
    /// Unreliable for fast presses
    device_state: DeviceState,

    is_caps_pressed: AtomicBool
}

impl AppKeyboardState {
    pub fn new() -> Option<Self> {
        if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(Self{ device_state: DeviceState {}, is_caps_pressed: AtomicBool::new(false) })
        } else {
            None
        }
    }
    
    pub fn is_caps_pressed(&self) -> bool {
        self.is_caps_pressed.load(Ordering::SeqCst)
    }

    pub fn handle_key_event(&self, key_event: &KeyEvent) {
        match key_event {
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                self.is_caps_pressed.store(true, Ordering::SeqCst);
            }
            KeyEvent::Release(Key(KeyMappingCode::CapsLock)) => {
                self.is_caps_pressed.store(false, Ordering::SeqCst);
            },
            _ => ()
        }
    }

    pub fn keys(&self) -> Vec<Key> {
        self.device_state.get_keys().into_iter().map(|k| Key::from(k)).collect()
    }
}
