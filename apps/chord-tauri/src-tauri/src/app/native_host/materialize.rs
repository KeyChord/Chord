//! Content-addressed materialization of a package's native artifact tree into Chord's cache.
//!
//! Imported packages are held in memory as bytes and never referenced by their original
//! location on disk. Dynamic loaders need real paths, and libraries may reference sibling
//! libraries through `@rpath`/`@loader_path`, so the whole `target/` subtree of a package is
//! written once under `<cache>/<sha256 of the tree>/` with its relative layout preserved.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MaterializedTree {
    pub sha256: String,
    /// Canonical directory containing the tree (`<cache>/<sha256>`).
    pub root: PathBuf,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

/// Hashes every `(relative path, bytes)` pair in a stable order.
pub fn tree_sha256<'a>(files: impl IntoIterator<Item = (&'a PathBuf, &'a Vec<u8>)>) -> String {
    let sorted: BTreeMap<_, _> = files.into_iter().collect();
    let mut hash = Sha256::new();
    for (path, bytes) in sorted {
        hash.update(path.to_string_lossy().as_bytes());
        hash.update([0]);
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    hex::encode(hash.finalize())
}

/// Writes `files` to `<cache_root>/<tree hash>/<relative path>` atomically (temp dir + rename)
/// unless a complete copy already exists. Returned paths are canonical so the host's cache-dir
/// check (`starts_with(cache_root)`) holds even when `cache_root` sits behind a symlink.
pub fn materialize_tree(
    cache_root: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<MaterializedTree> {
    for path in files.keys() {
        anyhow::ensure!(is_safe_relative(path), "invalid artifact path {path:?}");
    }
    std::fs::create_dir_all(cache_root)
        .with_context(|| format!("failed to create {}", cache_root.display()))?;

    let sha256 = tree_sha256(files.iter());
    let final_dir = cache_root.join(&sha256);
    let marker = final_dir.join(".complete");

    if !marker.exists() {
        let tmp_dir = cache_root.join(format!(".tmp-{}-{}", sha256, uuid::Uuid::new_v4()));
        let write_all = || -> Result<()> {
            for (path, bytes) in files {
                let target = tmp_dir.join(path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target, bytes)
                    .with_context(|| format!("failed to write {}", target.display()))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
                }
            }
            std::fs::write(tmp_dir.join(".complete"), b"")?;
            Ok(())
        };
        if let Err(error) = write_all() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(error);
        }

        match std::fs::rename(&tmp_dir, &final_dir) {
            Ok(()) => {}
            // Another import materialized the same tree concurrently.
            Err(_) if marker.exists() => {
                let _ = std::fs::remove_dir_all(&tmp_dir);
            }
            Err(error) => {
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return Err(error).with_context(|| {
                    format!("failed to move {} into place", final_dir.display())
                });
            }
        }
    }

    let root = std::fs::canonicalize(&final_dir)?;
    Ok(MaterializedTree { sha256, root })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(entries: &[(&str, &[u8])]) -> BTreeMap<PathBuf, Vec<u8>> {
        entries
            .iter()
            .map(|(p, b)| (PathBuf::from(p), b.to_vec()))
            .collect()
    }

    #[test]
    fn materializes_once_and_reuses() {
        let root = std::env::temp_dir().join(format!("chord-materialize-{}", uuid::Uuid::new_v4()));
        let files = tree(&[
            ("target/t/swift/menu/menu.dylib", b"library bytes"),
            ("target/t/swift/menu/M.swiftmodule", b"module"),
        ]);
        let first = materialize_tree(&root, &files).unwrap();
        let lib = first.root.join("target/t/swift/menu/menu.dylib");
        assert!(lib.exists());
        assert!(first.root.join("target/t/swift/menu/M.swiftmodule").exists());
        let mtime = std::fs::metadata(&lib).unwrap().modified().unwrap();

        let second = materialize_tree(&root, &files).unwrap();
        assert_eq!(first.root, second.root);
        assert_eq!(std::fs::metadata(&lib).unwrap().modified().unwrap(), mtime);

        let other = materialize_tree(&root, &tree(&[("target/t/swift/menu/menu.dylib", b"other")])).unwrap();
        assert_ne!(other.sha256, first.sha256);
        assert!(std::fs::read_dir(&root).unwrap().all(|e| {
            !e.unwrap().file_name().to_string_lossy().starts_with(".tmp-")
        }));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_unsafe_paths() {
        let root = std::env::temp_dir().join("chord-materialize-reject");
        assert!(materialize_tree(&root, &tree(&[("../x.dylib", b"x")])).is_err());
        assert!(materialize_tree(&root, &tree(&[("/abs/x.dylib", b"x")])).is_err());
    }

    #[test]
    fn tree_hash_is_order_independent() {
        let a = tree(&[("a", b"1"), ("b", b"2")]);
        let mut b = BTreeMap::new();
        b.insert(PathBuf::from("b"), b"2".to_vec());
        b.insert(PathBuf::from("a"), b"1".to_vec());
        assert_eq!(tree_sha256(a.iter()), tree_sha256(b.iter()));
    }
}
