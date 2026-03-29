use fast_radix_trie::StringRadixMap;

/// Mapping of all the relevant files for a chord package.
///
/// We intentionally don't include the path of the package here to avoid leaking implementation
/// details about where the package is located on the filesystem.
#[derive(Debug)]
pub struct RawChordPackage {
    /// The dirname is needed for inferring the chord package name when package.json isn't present
    pub dirname: String,
    pub package_json_contents: Option<String>,

    pub chords_files_contents: StringRadixMap<String>,
    pub js_files_contents: StringRadixMap<String>,
    pub bin_files_contents: StringRadixMap<Vec<u8>>,
}
