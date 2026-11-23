use crate::app::chord_package_manager::ChordReference;
use crate::define_observable;
use crate::models::{Chord, ChordHint, FilePathslug};
use serde::Serialize;
use std::collections::HashMap;
use typeshare::typeshare;

/// What the Hint Panel draws from, recomputed when the frontmost app or the packages
/// change. With the whole package state, some 3,000 chords took about 5 MB of the
/// panel's web content process, and every change to them reached it.
#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HintPanelState {
    /// Loaded packages in rank order, as in `ChordPackageManagerState::packages`.
    pub packages: Vec<HintPanelPackage>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HintPanelPackage {
    pub name: String,
    /// Every chords file with its hints. Only the frontmost app's files, and the Finder's
    /// while a file picker is open, keep their chords.
    pub chords_files: HashMap<FilePathslug, HintPanelChordsFile>,
    pub global_chords: Vec<ChordReference>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HintPanelChordsFile {
    pub chords: Vec<Chord>,
    pub chord_hints: Vec<ChordHint>,
}

define_observable!(
    #[derive(Debug)]
    pub struct HintPanelObservable(HintPanelState);
    id: "hint-panel";
);
