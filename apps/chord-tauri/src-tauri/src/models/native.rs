//! Layout of a chord package's prebuilt Node-API modules.
//!
//! A package's JS handler loads these through Bun's `process.dlopen`, and the `chord` module's
//! `resolveNativeModulePath(import.meta, relpath)` applies this layout so packages do not hardcode
//! target triples. Example: `src/swift/menu/menu.swift` -> `target/<triple>/menu/menu.node`.
use std::path::{Component, Path, PathBuf};

/// Top-level package directory holding platform-specific native build artifacts.
pub const NATIVE_TARGET_DIR: &str = "target";

/// The Rust-style target triple this build of Chord runs on, e.g. `aarch64-apple-darwin`.
/// Set by `build.rs` from cargo's `TARGET`.
pub const NATIVE_TARGET_TRIPLE: &str = env!("CHORD_TARGET_TRIPLE");

pub const NATIVE_MODULE_EXT: &str = "node";

/// A module path relative to `target/<triple>/`, e.g. `menu`.
/// Conservative on purpose: it becomes part of a filesystem path.
pub fn is_valid_native_module_relpath(relpath: &str) -> bool {
    if relpath.is_empty() || relpath.starts_with('/') || relpath.contains('\\') {
        return false;
    }
    let path = Path::new(relpath);
    if path.is_absolute() {
        return false;
    }
    for component in path.components() {
        match component {
            Component::Normal(segment) => {
                let Some(segment) = segment.to_str() else {
                    return false;
                };
                let Some(first) = segment.chars().next() else {
                    return false;
                };
                if !first.is_ascii_alphanumeric() {
                    return false;
                }
                if !segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                {
                    return false;
                }
                if segment == "." || segment == ".." {
                    return false;
                }
                if segment.ends_with(&format!(".{NATIVE_MODULE_EXT}")) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn native_module_stem(relpath: &str) -> Option<&str> {
    Path::new(relpath).file_stem()?.to_str()
}

/// `target/<triple>/<relpath>/<stem>.node` for this build's triple, relative to the
/// (logical) package root.
pub fn native_module_relpath(relpath: &str) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(
        is_valid_native_module_relpath(relpath),
        "invalid native module relpath {relpath:?} (expected e.g. \"menu\")"
    );
    let stem = native_module_stem(relpath).ok_or_else(|| {
        anyhow::anyhow!("invalid native module relpath {relpath:?} (missing filename)")
    })?;
    Ok(PathBuf::from(NATIVE_TARGET_DIR)
        .join(NATIVE_TARGET_TRIPLE)
        .join(relpath)
        .join(format!("{stem}.{NATIVE_MODULE_EXT}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_module_relpaths() {
        assert!(is_valid_native_module_relpath("menu"));
        assert!(is_valid_native_module_relpath("native/beep"));
    }

    #[test]
    fn rejects_invalid_relpaths() {
        assert!(!is_valid_native_module_relpath(""));
        assert!(!is_valid_native_module_relpath("../menu"));
        assert!(!is_valid_native_module_relpath("native/../menu"));
        assert!(!is_valid_native_module_relpath(".hidden/menu"));
        assert!(!is_valid_native_module_relpath(&format!(
            "menu.{NATIVE_MODULE_EXT}"
        )));
    }

    #[test]
    fn module_relpath_follows_the_layout() {
        let path = native_module_relpath("menu").unwrap();
        assert_eq!(
            path,
            PathBuf::from(format!(
                "target/{NATIVE_TARGET_TRIPLE}/menu/menu.{NATIVE_MODULE_EXT}"
            ))
        );
        assert!(native_module_relpath("../menu").is_err());
    }

    #[test]
    fn target_triple_is_set() {
        assert!(NATIVE_TARGET_TRIPLE.contains('-'), "{NATIVE_TARGET_TRIPLE}");
    }
}
