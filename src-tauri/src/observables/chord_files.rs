use crate::define_observable;
use serde::Serialize;
use std::collections::HashMap;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordFilesState {
    pub raw_files_as_json_strings: HashMap<String, String>,
}

define_observable!(
    pub struct ChordFilesObservable(ChordFilesState);
    id: "chord-files";
);
