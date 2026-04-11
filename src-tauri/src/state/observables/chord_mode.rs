use crate::define_observable;
use crate::models::{ChordInput, ChordInputEvent};
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordModeState {
    // Whether Shift is still held down for the current chord-mode interaction.
    pub is_shift_pressed: bool,

    /// Whether the chord indicator overlay should be shown while chord mode is active.
    pub is_indicator_visible: bool,
}

define_observable!(
    pub struct ChordModeObservable(ChordModeState);
    id: "chord-input";
);
