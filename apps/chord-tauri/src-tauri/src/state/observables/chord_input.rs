use crate::define_observable;
use crate::models::{ChordInput, ChordInputEvent};
use serde::Serialize;
use typeshare::typeshare;

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
    pub selected_input_event: Option<ChordInputEvent>,
}

define_observable!(
    pub struct ChordInputObservable(ChordInputState);
    id: "chord-input";
);
