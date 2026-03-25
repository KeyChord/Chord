use crate::app::chord_registry::chord_file::{
    AppChordMapValue, AppChordsFile, AppChordsFileConfig,
};
use crate::app::chord_runner::javascript::ChordJsInvocation;
use crate::app::chord_runner::shortcut::Shortcut;
use crate::app::placeholder_chord_store::{PlaceholderChordStoreEntry, PlaceholderChordStoreKey};
use crate::input::Key;
use anyhow::Result;
use rquickjs::{Object, Promise, Value};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub name: String,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
    pub js: Option<ChordJsInvocation>,
}

#[derive(Debug, Clone)]
pub struct MatchingChordInfo {
    pub scope: String,
    pub scope_kind: &'static str,
    pub sequence: Vec<Key>,
    pub chord: Chord,
}

#[derive(Debug, Clone)]
pub struct MatchingDescriptionInfo {
    pub scope: String,
    pub scope_kind: &'static str,
    pub sequence: Vec<Key>,
    pub description: String,
}

// Each chord runtime is associated with a JS module which lives in-memory
// (similar to require.cache)
pub struct ChordRuntime {
    // Used as a unique module key
    pub path: String,

    pub chords: HashMap<Vec<Key>, Chord>,
    pub descriptions: HashMap<Vec<Key>, String>,
    // Needs to be an Arc so the JS runtime can access its latest value
    pub raw_chords: Arc<Mutex<HashMap<String, AppChordMapValue>>>,
    pub config: Option<AppChordsFileConfig>,
}

#[derive(Debug, Clone)]
pub struct ChordPayload {
    pub chord: Chord,
    pub num_times: usize,
}

pub(crate) const GLOBAL_CHORD_RUNTIME_ID: &str = "__global__";

impl ChordRuntime {
    pub fn from_chords(path: String, chords: HashMap<Vec<Key>, Chord>) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(HashMap::new()));
        Ok(Self {
            path,
            chords,
            descriptions: HashMap::new(),
            raw_chords,
            config: None,
        })
    }

    // Doesn't resolve _config.extends
    pub fn from_file_shallow(
        path: String,
        chord_file: AppChordsFile,
        placeholder_bindings: &HashMap<String, String>,
    ) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(chord_file.get_raw_chords()));
        let config = chord_file.config.clone();

        // We intentionally keep global chords because they execute in this runtime
        let chords = chord_file.get_chords_shallow(placeholder_bindings);
        let descriptions = chord_file.get_descriptions_shallow();

        Ok(Self {
            path,
            raw_chords,
            config,
            chords,
            descriptions,
        })
    }

    pub fn extend_runtime(&mut self, base: &Self) -> Result<()> {
        for (sequence, chord) in &base.chords {
            self.chords
                .entry(sequence.clone())
                .or_insert_with(|| chord.clone());
        }

        for (sequence, description) in &base.descriptions {
            self.descriptions
                .entry(sequence.clone())
                .or_insert_with(|| description.clone());
        }

        let mut raw_chords = self.raw_chords.lock().expect("poisoned lock");
        let base_raw_chords = base.raw_chords.lock().expect("poisoned lock");
        for (sequence, chord) in base_raw_chords.iter() {
            raw_chords
                .entry(sequence.clone())
                .or_insert_with(|| chord.clone());
        }

        Ok(())
    }

    pub fn get_chord(&self, sequence: &[Key]) -> Option<ChordPayload> {
        let split_idx = sequence
            .iter()
            .position(|k| !k.is_digit())
            .unwrap_or(sequence.len());
        let (digit_keys, chord_keys) = sequence.split_at(split_idx);
        let num_times = if digit_keys.is_empty() {
            1
        } else {
            let digits: String = digit_keys.iter().filter_map(|k| k.to_char(false)).collect();
            let num_times = digits.parse::<usize>().unwrap_or(1);
            num_times
        };
        self.chords.get(chord_keys).map(|chord| ChordPayload {
            chord: chord.clone(),
            num_times,
        })
    }
}

