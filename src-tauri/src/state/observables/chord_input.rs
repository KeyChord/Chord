use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;
use crate::models::{ChordInput, ChordInputEvent};

#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordInputState {
    /// The keys a user has pressed for a chord but not yet executed. Think of it as the equivalent
    /// of an HTML <input />'s `value`. Always cleared when executed.
    pub input: ChordInput,

    /// The chord currently being pressed down.
    pub pressed_input_event: Option<ChordInputEvent>,

    /// The chord that is "active"
    pub loaded_input_event: Option<ChordInputEvent>,

    // Whether Shift is still held down for the current chord-mode interaction.
    pub is_shift_pressed: bool,

    // Whether the chord indicator overlay should be shown while chord mode is active.
    pub is_indicator_visible: bool,
}

define_observable!(
    pub struct ChordInputObservable(ChordInputState);
    id: "chord-input";
);
