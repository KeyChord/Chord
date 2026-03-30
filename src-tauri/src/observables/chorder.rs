use crate::define_observable;
use crate::input::Key;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChorderState {
    // The key buffer represents the pending letters for a not-yet created chord.
    // When a chord is executed, the key buffer is always cleared.
    pub key_buffer: Vec<Key>,

    // The chord currently being pressed down.
    pub pressed_chord_keys: Option<Vec<Key>>,

    // The chord that is "active"
    pub active_chord_keys: Option<Vec<Key>>,

    // Whether Shift is still held down for the current chord-mode interaction.
    pub is_shift_pressed: bool,

    // Whether the chord indicator overlay should be shown while chord mode is active.
    pub is_indicator_visible: bool,
}

impl ChorderState {
    pub fn is_idle(&self) -> bool {
        self.key_buffer.is_empty()
            && self.pressed_chord_keys.is_none()
            && self.active_chord_keys.is_none()
    }

    pub fn clear_session(&self) -> Self {
        Self {
            key_buffer: vec![],
            pressed_chord_keys: None,
            active_chord_keys: None,
            is_shift_pressed: false,
            is_indicator_visible: self.is_indicator_visible,
        }
    }

    pub fn with_session(
        &self,
        key_buffer: Vec<Key>,
        pressed_chord_keys: Option<Vec<Key>>,
        active_chord_keys: Option<Vec<Key>>,
    ) -> Self {
        Self {
            key_buffer,
            pressed_chord_keys,
            active_chord_keys,
            is_shift_pressed: self.is_shift_pressed,
            is_indicator_visible: self.is_indicator_visible,
        }
    }

    pub fn with_shift_pressed(&self, is_shift_pressed: bool) -> Self {
        Self {
            is_shift_pressed,
            ..self.clone()
        }
    }

    pub fn toggled_indicator_visibility(&self) -> Self {
        Self {
            is_indicator_visible: !self.is_indicator_visible,
            ..self.clone()
        }
    }
}

impl Default for ChorderState {
    fn default() -> Self {
        Self {
            key_buffer: vec![],
            pressed_chord_keys: None,
            active_chord_keys: None,
            is_shift_pressed: false,
            is_indicator_visible: true,
        }
    }
}

define_observable!(
    pub struct ChorderObservable(ChorderState);
    id: "chorder";
);
