use super::{Key, ChordAction, ChordTrigger};
use serde::Serialize;
use typeshare::typeshare;

/// The raw keys the user inputs to match a Chord trigger
#[typeshare]
pub type ChordInput = Vec<Key>;

#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordInputEvent {
    /// The sequence of keys the user pressed to trigger the chord.
    pub input: ChordInput,

    /// The ID of the application the chord execution applied to.
    pub application_id: Option<String>,
}

/// A mapping from a sequence (or pattern) of keys to a series of actions.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chord {
    pub raw_trigger: String,

    /// The keys that make up the chord (extracted from the TOML key)
    #[typeshare(typescript(type = "{ keys: string[] } | { regex: string }"))]
    pub trigger: ChordTrigger,

    /// A mandatory chord name
    pub name: String,

    /// The relative index of the chord inside the TOML file
    pub index: u32,

    /// A list of actions (as fallbacks) to execute when the chord is triggered
    pub actions: Vec<ChordAction>,
}

/// The chord equivalent of `onKeyPress`.
pub struct ChordPressEvent {
    /// The sequence of keys the user pressed to trigger the chord.
    pub input: ChordInput,

    /// The chord whose trigger matched the input.
    pub chord: Chord,

    /// The ID of the application the chord execution applied to.
    pub application_id: Option<String>,
}
