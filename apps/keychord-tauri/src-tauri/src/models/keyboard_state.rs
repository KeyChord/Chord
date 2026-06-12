use std::sync::atomic::{AtomicBool, Ordering};
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use nject::injectable;
use crate::models::{Key, KeyEvent};
use crate::state::KeyboardObservable;

#[injectable]
pub struct AppKeyboardState {
    #[inject(
        if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(DeviceState {})
        } else {
            None
        }
    )]
    device_state: Option<DeviceState>,
    #[inject(AtomicBool::new(false))]
    is_caps_pressed: AtomicBool,
    #[inject(AtomicBool::new(false))]
    is_shift_pressed: AtomicBool,

    observable: KeyboardObservable
}

impl AppKeyboardState {
    pub fn is_caps_pressed(&self) -> bool {
        self.is_caps_pressed.load(Ordering::SeqCst)
    }

    pub fn is_shift_pressed(&self) -> bool {
        self.is_shift_pressed.load(Ordering::SeqCst)
    }

    pub fn handle_key_event(&self, key_event: &KeyEvent) {
        match key_event {
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                self.is_caps_pressed.store(true, Ordering::SeqCst);
            }
            KeyEvent::Release(Key(KeyMappingCode::CapsLock)) => {
                self.is_caps_pressed.store(false, Ordering::SeqCst);
            },

            KeyEvent::Press(Key(KeyMappingCode::ShiftLeft | KeyMappingCode::ShiftRight)) => {
                self.is_shift_pressed.store(true, Ordering::SeqCst);
            }
            KeyEvent::Release(Key(KeyMappingCode::ShiftLeft | KeyMappingCode::ShiftRight)) => {
                self.is_shift_pressed.store(false, Ordering::SeqCst);
            },
            _ => ()
        }
    }

    pub fn keys(&self) -> Option<Vec<Key>> {
        Some(self.device_state.as_ref()?.get_keys().into_iter().map(|k| Key::from(k)).collect())
    }
}
