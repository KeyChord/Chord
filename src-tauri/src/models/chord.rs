use serde::Serialize;
use typeshare::typeshare;
use crate::models::{ChordAction, ChordTrigger};

/// A regular chord entry composed of static characters
#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct Chord {
    /// The keys that make up the chord (extracted from the TOML key)
    pub trigger: ChordTrigger,

    /// A mandatory chord name
    pub name: String,

    /// The relative index of the chord inside the TOML file
    pub index: u32,

    /// A list of actions (as fallbacks) to execute when the chord is triggered
    pub actions: Vec<ChordAction>,
}

