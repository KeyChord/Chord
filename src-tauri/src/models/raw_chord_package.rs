use std::collections::HashMap;
use std::path::PathBuf;
use serde::Serialize;
use typeshare::typeshare;

/// Mapping of all the relevant files for a chord package.
///
/// We intentionally don't include the path of the package here to avoid leaking implementation
/// details about where the package is located on the filesystem.
#[typeshare]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawChordPackage {
    /// The dirname is needed for inferring the chord package name when package.json isn't present
    pub dirname: String,
    pub package_json_contents: Option<String>,

    pub chords_files_contents: HashMap<PathBuf, String>,
    pub js_files_contents: HashMap<PathBuf, String>,
    pub bin_files_contents: HashMap<PathBuf, Vec<u8>>,
}
