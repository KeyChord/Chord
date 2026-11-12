use super::{ChordAction, ChordTrigger, Key};
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

    /// Text typed in insert mode for each quote in `input`, in order. Such a quote
    /// appears in `input` as an adjacent `''` pair.
    pub quotes: Vec<String>,
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

    /// A catch-all, such as the `/(.*)` launcher: it runs only when no ordinary chord matches,
    /// whichever packages define them.
    pub fallback: bool,
}

impl Chord {
    pub fn description(&self, keys: &[Key]) -> String {
        if let ChordTrigger::Pattern(pattern) = &self.trigger {
            if let Some(input) = Key::serialize_sequence(keys) {
                if let Some(captures) = pattern.captures(&input) {
                    let mut description = String::new();
                    captures.expand(&self.name, &mut description);
                    return description;
                }
            }
        }
        self.name.clone()
    }
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
