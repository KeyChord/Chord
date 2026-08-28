//! Layout of a chord package's prebuilt native libraries.
//!
//! Chord never loads these itself: a package's JS handler opens them with `bun:ffi`, and the
//! `chord` module's `resolveNativeLibrary(import.meta, name)` applies this layout so packages do
//! not hardcode paths. Anything that produces `target/<triple>/native/<name>/<name>.<ext>` works
//! (the `@keychord/config` build tooling does).
use std::path::PathBuf;

/// Top-level package directory holding platform-specific native build artifacts.
pub const NATIVE_TARGET_DIR: &str = "target";

/// Subdirectory of `target/<triple>/` holding compiled native modules.
pub const NATIVE_MODULE_SUBDIR: &str = "native";

/// The Rust-style target triple this build of Chord runs on, e.g. `aarch64-apple-darwin`.
/// Set by `build.rs` from cargo's `TARGET`.
pub const NATIVE_TARGET_TRIPLE: &str = env!("CHORD_TARGET_TRIPLE");

#[cfg(target_os = "macos")]
pub const NATIVE_LIBRARY_EXT: &str = "dylib";
#[cfg(target_os = "windows")]
pub const NATIVE_LIBRARY_EXT: &str = "dll";
#[cfg(all(unix, not(target_os = "macos")))]
pub const NATIVE_LIBRARY_EXT: &str = "so";

/// A logical native module name (`menu` -> `target/<triple>/native/menu/menu.dylib`).
/// Conservative on purpose: it becomes part of a filesystem path.
pub fn is_valid_native_target_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        return false;
    }
    if name.ends_with(&format!(".{NATIVE_LIBRARY_EXT}")) || name == "." || name == ".." {
        return false;
    }
    true
}

/// `target/<triple>/native/<name>/<name>.<ext>` for this build's triple, relative to the
/// (logical) package root.
pub fn native_library_relpath(name: &str) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(
        is_valid_native_target_name(name),
        "invalid native module name {name:?} (expected e.g. \"menu\")"
    );
    Ok(PathBuf::from(NATIVE_TARGET_DIR)
        .join(NATIVE_TARGET_TRIPLE)
        .join(NATIVE_MODULE_SUBDIR)
        .join(name)
        .join(format!("{name}.{NATIVE_LIBRARY_EXT}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_names() {
        assert!(is_valid_native_target_name("menu"));
        assert!(is_valid_native_target_name("paste-date"));
        assert!(is_valid_native_target_name("v2_beta.x"));
    }

    #[test]
    fn rejects_paths_and_extensions() {
        assert!(!is_valid_native_target_name(""));
        assert!(!is_valid_native_target_name("../menu"));
        assert!(!is_valid_native_target_name("sub/menu"));
        assert!(!is_valid_native_target_name(".hidden"));
        assert!(!is_valid_native_target_name(&format!("menu.{NATIVE_LIBRARY_EXT}")));
    }

    #[test]
    fn library_relpath_follows_the_layout() {
        let path = native_library_relpath("menu").unwrap();
        assert_eq!(
            path,
            PathBuf::from(format!("target/{NATIVE_TARGET_TRIPLE}/native/menu/menu.{NATIVE_LIBRARY_EXT}"))
        );
        assert!(native_library_relpath("../menu").is_err());
    }

    #[test]
    fn target_triple_is_set() {
        assert!(NATIVE_TARGET_TRIPLE.contains('-'), "{NATIVE_TARGET_TRIPLE}");
    }
}
