use crate::app::native_host::materialize::{materialize_tree, sha256_hex};
use crate::models::{
    FilePathslug, NATIVE_LIBRARY_EXT, NATIVE_MODULE_SUBDIR, NATIVE_TARGET_DIR, NATIVE_TARGET_TRIPLE,
    is_valid_native_target_name,
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use typeshare::typeshare;

/// The validated, materialized native artifacts of one package. Mirrors `ChordJsPackage`:
/// libraries are keyed by pathslug (`target/<triple>/native/menu/menu.dylib`) and resolved
/// relative to the chords file that references them so bundled (nested) packages resolve to
/// their own libraries. The whole `target/` subtree is materialized together so libraries can
/// reference vendored sibling libraries via `@loader_path`.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordNativePackage {
    pub name: String,
    /// Hash of the materialized artifact tree.
    pub tree_sha256: String,
    libraries: HashMap<FilePathslug, NativeLibraryArtifact>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeLibraryArtifact {
    pub sha256: String,
    #[typeshare(serialized_as = "u32")]
    pub size: u64,
    /// Where the bytes live in Chord's cache. Internal; not shared with the frontend.
    #[serde(skip)]
    pub materialized_path: PathBuf,
}

impl ChordNativePackage {
    /// Materializes the package's `target/` files (already filtered to the current triple by the
    /// registry) into `cache_root` and indexes every handler library found in the tree.
    pub fn load(
        name: &str,
        files: HashMap<FilePathslug, Vec<u8>>,
        cache_root: &Path,
    ) -> Result<Self> {
        let files: BTreeMap<PathBuf, Vec<u8>> = files.into_iter().collect();
        let tree = materialize_tree(cache_root, &files)
            .with_context(|| format!("failed to materialize native artifacts of package {name}"))?;

        let mut libraries = HashMap::new();
        for (pathslug, bytes) in &files {
            if pathslug.extension().and_then(|e| e.to_str()) != Some(NATIVE_LIBRARY_EXT) {
                continue;
            }
            let materialized_path = tree.root.join(pathslug);
            log::debug!(
                "native library {:?} of {} ({} bytes) at {}",
                pathslug,
                name,
                bytes.len(),
                materialized_path.display()
            );
            libraries.insert(
                pathslug.clone(),
                NativeLibraryArtifact {
                    sha256: sha256_hex(bytes),
                    size: bytes.len() as u64,
                    materialized_path,
                },
            );
        }
        Ok(Self {
            name: name.to_string(),
            tree_sha256: tree.sha256,
            libraries,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.libraries.is_empty()
    }

    /// Resolves a logical handler name declared in a chords file to a materialized library.
    /// "menu" from chords/com/app/macos.toml
    ///   -> target/<triple>/native/menu/menu.dylib
    /// "menu" from chords/@pkg/name/chords/com/app/macos.toml
    ///   -> target/@pkg/name/target/<triple>/native/menu/menu.dylib
    pub fn resolve_file(
        &self,
        chords_file_pathslug: &FilePathslug,
        file: &str,
    ) -> Result<Option<&NativeLibraryArtifact>> {
        anyhow::ensure!(
            is_valid_native_target_name(file),
            "invalid native handler name {file:?}: expected a plain module name without \
             directories or the .{NATIVE_LIBRARY_EXT} extension"
        );
        let pathslug = Self::library_pathslug(chords_file_pathslug, file)?;
        Ok(self.libraries.get(&pathslug))
    }

    pub fn library_pathslug(chords_file_pathslug: &Path, file: &str) -> Result<PathBuf> {
        let components: Vec<_> = chords_file_pathslug.components().collect();
        let root = if components.len() >= 4
            && components[0].as_os_str() == "chords"
            && components[1]
                .as_os_str()
                .to_str()
                .unwrap_or("")
                .starts_with('@')
            && components[3].as_os_str() == "chords"
        {
            Path::new(NATIVE_TARGET_DIR)
                .join(components[1].as_os_str())
                .join(components[2].as_os_str())
                .join(NATIVE_TARGET_DIR)
        } else if components.first().map(|c| c.as_os_str()) == Some(std::ffi::OsStr::new("chords"))
        {
            PathBuf::from(NATIVE_TARGET_DIR)
        } else {
            anyhow::bail!("Path does not match a recognized chords directory structure");
        };
        Ok(root
            .join(NATIVE_TARGET_TRIPLE)
            .join(NATIVE_MODULE_SUBDIR)
            .join(file)
            .join(format!("{file}.{NATIVE_LIBRARY_EXT}")))
    }

    pub fn library_pathslugs(&self) -> impl Iterator<Item = &FilePathslug> {
        self.libraries.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(prefix: &str, name: &str) -> PathBuf {
        PathBuf::from(format!(
            "{prefix}{NATIVE_TARGET_DIR}/{NATIVE_TARGET_TRIPLE}/{NATIVE_MODULE_SUBDIR}/{name}/{name}.{NATIVE_LIBRARY_EXT}"
        ))
    }

    fn package() -> ChordNativePackage {
        let root = std::env::temp_dir().join(format!("chord-native-pkg-{}", uuid::Uuid::new_v4()));
        let mut files = HashMap::new();
        files.insert(lib("", "menu"), b"top".to_vec());
        files.insert(
            PathBuf::from(format!(
                "{NATIVE_TARGET_DIR}/{NATIVE_TARGET_TRIPLE}/{NATIVE_MODULE_SUBDIR}/menu/Module.swiftmodule"
            )),
            b"module".to_vec(),
        );
        files.insert(lib("target/@keychord/chords-menu/", "menu"), b"nested".to_vec());
        ChordNativePackage::load("pkg", files, &root).unwrap()
    }

    #[test]
    fn resolves_standard_and_nested_layouts() {
        let package = package();
        let top = package
            .resolve_file(&PathBuf::from("chords/com/apple/Safari/macos.toml"), "menu")
            .unwrap()
            .unwrap();
        let nested = package
            .resolve_file(
                &PathBuf::from("chords/@keychord/chords-menu/chords/macos.toml"),
                "menu",
            )
            .unwrap()
            .unwrap();
        assert_ne!(top.sha256, nested.sha256);
        assert!(top.materialized_path.exists());
        assert!(top
            .materialized_path
            .parent()
            .unwrap()
            .join("Module.swiftmodule")
            .exists(), "module files are materialized next to the library");
        assert!(package
            .resolve_file(&PathBuf::from("chords/macos.toml"), "missing")
            .unwrap()
            .is_none());
        assert_eq!(package.library_pathslugs().count(), 2, "non-library files are not handlers");
    }

    #[test]
    fn rejects_invalid_names() {
        let package = package();
        let pathslug = PathBuf::from("chords/macos.toml");
        assert!(package.resolve_file(&pathslug, "../menu").is_err());
        assert!(package.resolve_file(&pathslug, "sub/menu").is_err());
        assert!(package
            .resolve_file(&pathslug, &format!("menu.{NATIVE_LIBRARY_EXT}"))
            .is_err());
        assert!(package.resolve_file(&PathBuf::from("js/x.toml"), "menu").is_err());
    }
}
