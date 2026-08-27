/// Top-level package directory holding platform-specific native build artifacts:
/// `target/<triple>/native/<name>/<name>.<ext>` plus the compiled module files other packages
/// import. Source language is irrelevant to Chord: anything that links into a library exporting
/// `chord_native_run_v1` is a native handler, so the directory names the artifact kind, not a
/// language.
pub const NATIVE_TARGET_DIR: &str = "target";

/// Subdirectory of `target/<triple>/` holding compiled native handler modules.
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

/// A logical native handler name as written in `[on.<event>] file = "..."`.
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

/// Whether the current process runs inside the macOS App Sandbox, in which case native handlers
/// cannot offer the unrestricted access they promise.
pub fn is_app_sandboxed() -> bool {
    std::env::var_os("APP_SANDBOX_CONTAINER_ID").is_some()
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
    fn target_triple_is_set() {
        assert!(NATIVE_TARGET_TRIPLE.contains('-'), "{NATIVE_TARGET_TRIPLE}");
    }
}
