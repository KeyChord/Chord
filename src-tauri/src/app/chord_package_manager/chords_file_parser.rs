use std::collections::HashMap;
use serde::Serialize;
use crate::models::{Chord, ChordHint};

/// A chords file is a TOML file
struct ChordsFileParser {}

#[derive(Debug, Serialize)]
struct ParsedChordsFile {
    name: String,
    meta: HashMap<String, String>,
    js: HashMap<String, String>,
    
    chords: Vec<Chord>,
    chord_hints: Vec<ChordHint>
}
