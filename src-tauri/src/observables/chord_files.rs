use crate::define_observable;
use serde::Serialize;
use std::collections::HashMap;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderChordInfo {
    pub file_path: String,
    pub scope: String,
    pub scope_kind: String,
    pub name: String,
    pub placeholder: String,
    pub sequence_template: String,
    pub sequence_prefix: String,
    pub sequence_suffix: String,
    pub assigned_sequence: Option<String>,
}

#[typeshare]
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordFilesState {
    pub raw_files_as_json_strings: HashMap<String, String>,
    pub placeholder_chords: Vec<PlaceholderChordInfo>,
}

define_observable!(
    pub struct ChordFilesObservable(ChordFilesState);
    id: "chord-files";
);
