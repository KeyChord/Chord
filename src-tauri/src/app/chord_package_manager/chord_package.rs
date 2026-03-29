use std::collections::HashMap;
use crate::app::chord_js_package_registry::ChordJsPackage;
use crate::input::Key;
use crate::models::{Chord, ChordInput, ChordString};
use crate::models::chords_file::ChordsFile;

type AppBundleId = String;

#[derive(Clone)]
pub struct ChordPackage {
    /// The `name` property of the `package.json` file; defaults to the folder name if not present.
    pub name: String,

    pub js_package: Option<ChordJsPackage>,

    pub app_chords_files: HashMap<AppBundleId, ChordsFile>,
    pub global_chords: HashMap<ChordString, Chord>
}

impl ChordPackage {
    pub fn resolve_chord_for_input(&self, input: &ChordInput) -> Option<&Chord> {
        if let Some(app_id) = &input.application_id {
            if let Some(chords_file) = self.app_chords_files.get(app_id) {
                if let Some(chord) = chords_file.chords.iter().find(|c| c.trigger.matches(&input.keys)) {
                    return Some(chord);
                }
            }
        }

        self.global_chords.values().find(|c| c.trigger.matches(&input.keys))
    }
}