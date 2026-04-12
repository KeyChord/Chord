use crate::define_observable;
use crate::models::{ChordInput, ChordInputEvent};
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPanelState {
    pub is_visible: bool,
}

define_observable!(
    pub struct ChordPanelObservable(ChordPanelState);
    id: "chord-panel";
);
