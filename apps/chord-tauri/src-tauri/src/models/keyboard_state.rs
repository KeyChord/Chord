use std::sync::atomic::{AtomicBool, Ordering};
use keycode::KeyMappingCode;
use nject::injectable;
use crate::models::{Key, KeyEvent};
use crate::state::KeyboardObservable;

#[injectable]
pub struct AppKeyboardState {
    #[inject(AtomicBool::new(false))]
    is_caps_pressed: AtomicBool,
    /// Whether the held Caps press starts Chord when Space follows; see `arm_caps`.
    #[inject(AtomicBool::new(false))]
    is_caps_armed: AtomicBool,
    #[inject(AtomicBool::new(false))]
    is_shift_pressed: AtomicBool,

    observable: KeyboardObservable
}

impl AppKeyboardState {
    pub fn reset(&self) {
        self.is_caps_pressed.store(false, Ordering::SeqCst);
        self.is_caps_armed.store(false, Ordering::SeqCst);
        self.is_shift_pressed.store(false, Ordering::SeqCst);
    }

    pub fn is_caps_pressed(&self) -> bool {
        self.is_caps_pressed.load(Ordering::SeqCst)
    }

    /// Caps is held and Space would start Chord.
    pub fn is_caps_armed(&self) -> bool {
        self.is_caps_armed.load(Ordering::SeqCst)
    }

    /// Records the Caps Lock state from just before the Caps press in progress. With
    /// Caps Lock on, the press turns it off and nothing else: someone typing in
    /// capitals who follows that with a space gets the space, not Chord.
    pub fn arm_caps(&self, caps_lock_was_on: bool) {
        self.is_caps_armed.store(!caps_lock_was_on, Ordering::SeqCst);
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
                self.is_caps_armed.store(false, Ordering::SeqCst);
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
}
