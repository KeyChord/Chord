use std::sync::atomic::{AtomicBool, Ordering};
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use nject::injectable;
use crate::models::{Key, KeyEvent};
use crate::state::KeyboardObservable;

#[injectable]
pub struct AppKeyboardState {
    #[inject(AtomicBool::new(false))]
    is_caps_pressed: AtomicBool,
    #[inject(AtomicBool::new(false))]
    is_shift_pressed: AtomicBool,

    observable: KeyboardObservable
}

impl AppKeyboardState {
    pub fn reset(&self) {
        self.is_caps_pressed.store(false, Ordering::SeqCst);
        self.is_shift_pressed.store(false, Ordering::SeqCst);
    }

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

    /// The keys held right now. Only key events call this, and they only arrive once the event
    /// tap runs, so Accessibility is granted by then. It must not be checked once at startup: a
    /// new user grants it while Chord is running, and every key they typed would be ignored.
    pub fn keys(&self) -> Option<Vec<Key>> {
        Some(DeviceState {}.get_keys().into_iter().map(|k| Key::from(k)).collect())
    }
}
