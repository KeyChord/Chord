use super::FilePathslug;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use typeshare::typeshare;

/// Mapping of all the relevant files for a chord package.
///
/// The on-disk location is kept out of the serialized form so the frontend never learns
/// where a package is installed; the engines need it (`root`) to import modules from disk.
#[typeshare]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawChordPackage {
    /// The dirname is needed for inferring the chord package name when package.json isn't present
    pub dirname: String,
    /// Absolute path of the package directory.
    #[serde(skip)]
    #[typeshare(skip)]
    pub root: PathBuf,
    pub package_json_contents: Option<String>,

    pub chords_files_contents: HashMap<FilePathslug, String>,
    pub js_files_contents: HashMap<FilePathslug, String>,
    pub bin_files_contents: HashMap<FilePathslug, Vec<u8>>,
}

impl RawChordPackage {
    pub fn package_name(&self) -> String {
        if let Ok(Some(name)) = self.get_package_name_from_package_json().inspect_err(|e| {
            log::error!("failed to infer package name from package.json, falling back to dirname")
        }) {
            name
        } else {
            self.dirname.clone()
        }
    }

    fn get_package_name_from_package_json(&self) -> anyhow::Result<Option<String>> {
        if let Some(package_json_contents) = &self.package_json_contents {
            let json: serde_json::Value = serde_json::from_str(package_json_contents)?;
            Ok(json
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }
}
