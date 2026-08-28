//! Layout of a chord package's prebuilt native libraries.
//!
//! Chord never loads these itself: a package's JS handler opens them with `bun:ffi`, and the
//! `chord` module's `resolveFfiPath(import.meta, relpath)` applies this layout so packages do
//! not hardcode paths. Anything that produces
//! `target/<triple>/<dir-relpath>/<filename>/<stem>.<ext>` works (the `@keychord/config` build
//! tooling does). Example: `src/ffi/menu.swift` -> `target/<triple>/ffi/menu.swift/menu.dylib`.
use std::path::{Component, Path, PathBuf};

/// Top-level package directory holding platform-specific native build artifacts.
pub const NATIVE_TARGET_DIR: &str = "target";

/// The Rust-style target triple this build of Chord runs on, e.g. `aarch64-apple-darwin`.
/// Set by `build.rs` from cargo's `TARGET`.
pub const NATIVE_TARGET_TRIPLE: &str = env!("CHORD_TARGET_TRIPLE");

#[cfg(target_os = "macos")]
pub const NATIVE_LIBRARY_EXT: &str = "dylib";
#[cfg(target_os = "windows")]
pub const NATIVE_LIBRARY_EXT: &str = "dll";
#[cfg(all(unix, not(target_os = "macos")))]
pub const NATIVE_LIBRARY_EXT: &str = "so";

/// A module path relative to `target/<triple>/`, e.g. `ffi/menu.swift`.
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
                if segment.ends_with(&format!(".{NATIVE_LIBRARY_EXT}")) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn native_library_stem(relpath: &str) -> Option<&str> {
    Path::new(relpath).file_stem()?.to_str()
}

/// `target/<triple>/<relpath>/<stem>.<ext>` for this build's triple, relative to the
/// (logical) package root.
pub fn native_library_relpath(relpath: &str) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(
        is_valid_native_module_relpath(relpath),
        "invalid native module relpath {relpath:?} (expected e.g. \"ffi/menu.swift\")"
    );
    let stem = native_library_stem(relpath).ok_or_else(|| {
        anyhow::anyhow!("invalid native module relpath {relpath:?} (missing filename)")
    })?;
    Ok(PathBuf::from(NATIVE_TARGET_DIR)
        .join(NATIVE_TARGET_TRIPLE)
        .join(relpath)
        .join(format!("{stem}.{NATIVE_LIBRARY_EXT}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_module_relpaths() {
        assert!(is_valid_native_module_relpath("ffi/menu.swift"));
        assert!(is_valid_native_module_relpath("ffi/sub/beep.swift"));
    }

    #[test]
    fn rejects_invalid_relpaths() {
        assert!(!is_valid_native_module_relpath(""));
        assert!(!is_valid_native_module_relpath("../menu.swift"));
        assert!(!is_valid_native_module_relpath("ffi/../menu.swift"));
        assert!(!is_valid_native_module_relpath(".hidden/menu.swift"));
        assert!(!is_valid_native_module_relpath(&format!(
            "ffi/menu.{NATIVE_LIBRARY_EXT}"
        )));
    }

    #[test]
    fn library_relpath_follows_the_layout() {
        let path = native_library_relpath("ffi/menu.swift").unwrap();
        assert_eq!(
            path,
            PathBuf::from(format!(
                "target/{NATIVE_TARGET_TRIPLE}/ffi/menu.swift/menu.{NATIVE_LIBRARY_EXT}"
            ))
        );
        assert!(native_library_relpath("../menu.swift").is_err());
    }

    #[test]
    fn target_triple_is_set() {
        assert!(NATIVE_TARGET_TRIPLE.contains('-'), "{NATIVE_TARGET_TRIPLE}");
    }
}
