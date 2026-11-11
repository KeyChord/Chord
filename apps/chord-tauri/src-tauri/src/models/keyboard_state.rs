use std::sync::atomic::{AtomicBool, Ordering};
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use nject::injectable;
use parking_lot::Mutex;
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
    /// The keys held according to the events Chord received, kept while the OS
    /// key-state query cannot be trusted; see `track_keys`.
    #[inject(Mutex::new(None))]
    tracked_keys: Mutex<Option<Vec<Key>>>,

    observable: KeyboardObservable
}

impl AppKeyboardState {
    pub fn reset(&self) {
        self.is_caps_pressed.store(false, Ordering::SeqCst);
        self.is_caps_armed.store(false, Ordering::SeqCst);
        self.is_shift_pressed.store(false, Ordering::SeqCst);
        *self.tracked_keys.lock() = None;
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
        if let Some(keys) = self.tracked_keys.lock().as_mut() {
            match key_event {
                KeyEvent::Press(key) => {
                    if !keys.contains(key) {
                        keys.push(*key);
                    }
                }
                KeyEvent::Release(key) => keys.retain(|held| held != key),
            }
        }
    }

    /// Starts keeping the held keys from Chord's own events, beginning with `held`.
    /// Under Secure Input the OS key-state query reports nothing as held, so a chord
    /// read through Chord's key window (see `key_catcher`) cannot use it.
    pub fn track_keys(&self, held: Vec<Key>) {
        *self.tracked_keys.lock() = Some(held);
    }

    pub fn stop_tracking_keys(&self) {
        *self.tracked_keys.lock() = None;
    }

    /// The keys held right now. Only key events call this, and they only arrive once the event
    /// tap runs, so Accessibility is granted by then. It must not be checked once at startup: a
    /// new user grants it while Chord is running, and every key they typed would be ignored.
    pub fn keys(&self) -> Option<Vec<Key>> {
        if let Some(keys) = self.tracked_keys.lock().as_ref() {
            return Some(keys.clone());
        }
        Some(DeviceState {}.get_keys().into_iter().map(|k| Key::from(k)).collect())
    }
}
