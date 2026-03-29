use std::collections::HashMap;
use crate::app::chord_js_package_registry::ChordJsPackage;
use crate::input::Key;
use crate::models::{Chord, ChordString};
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
    pub fn get_chord(&self, app_id: &Option<String>, keys: &[Key]) -> Option<&Chord> {
        None
    }
}