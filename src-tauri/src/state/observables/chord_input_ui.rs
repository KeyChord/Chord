use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;
use crate::models::{ChordInput, ChordInputEvent};

#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordInputUiState {
    // Whether the chord indicator overlay should be shown while chord mode is active.
    pub is_visible: bool,
}

define_observable!(
    pub struct ChordInputUiObservable(ChordInputUiState);
    id: "chord-input-ui";
);
