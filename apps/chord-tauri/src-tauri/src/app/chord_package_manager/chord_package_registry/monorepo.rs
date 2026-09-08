use super::LocalPackageRegistry;
use crate::models::RawChordPackage;
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Only immediate packages/chords-* directories are chord packages.
pub fn discover_monorepo_packages(root: &Path) -> Result<Vec<PathBuf>> {
    let packages_dir = root.join("packages");
    let mut paths = Vec::new();
    for entry in fs::read_dir(&packages_dir).with_context(|| {
        format!(
            "Cannot read monorepo packages directory {}",
            packages_dir.display()
        )
    })? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with("chords-") && entry.path().is_dir() {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn validate_monorepo_path(path: &str) -> Result<PathBuf> {
    ensure!(!path.trim().is_empty(), "Monorepo path cannot be empty");
    let root = fs::canonicalize(path.trim()).context("Failed to access monorepo folder")?;
    ensure!(
        !discover_monorepo_packages(&root)?.is_empty(),
        "No chord packages found. Expected folders under packages/chords-* in {}",
        root.display()
    );
    // Detect unreadable packages and duplicate names before saving the source.
    import_monorepo_packages(&root)?;
    Ok(root)
}

pub fn import_monorepo_packages(root: &Path) -> Result<HashMap<String, RawChordPackage>> {
    let mut packages = HashMap::new();
    for path in discover_monorepo_packages(root)? {
        let package = LocalPackageRegistry::import_from_local_folder(&path)?;
        let name = package.package_name();
        ensure!(
            !packages.contains_key(&name),
            "Duplicate package {name} in monorepo {}",
            root.display()
        );
        packages.insert(name, package);
    }
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("chord-monorepo-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(root.join("packages")).unwrap();
            Self(root)
        }
        fn package(&self, folder: &str, name: &str) -> PathBuf {
            let path = self.0.join("packages").join(folder);
            fs::create_dir_all(path.join("chords")).unwrap();
            fs::write(
                path.join("package.json"),
                serde_json::json!({"name": name}).to_string(),
            )
            .unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn monorepo_discovers_only_immediate_matching_directories() {
        let fixture = Fixture::new();
        let first = fixture.package("chords-a", "@test/a");
        let last = fixture.package("chords-z", "@test/z");
        fixture.package("utility", "@test/ignored");
        fixture.package("group/chords-nested", "@test/nested");
        fs::write(fixture.0.join("packages/chords-file"), "not a folder").unwrap();
        assert_eq!(
            discover_monorepo_packages(&fixture.0).unwrap(),
            vec![first.clone(), last]
        );
        let packages = import_monorepo_packages(&fixture.0).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages["@test/a"].root, first);
        assert!(validate_monorepo_path(fixture.0.to_str().unwrap()).is_ok());
    }

    #[test]
    fn monorepo_reload_picks_up_edits_additions_and_removals() {
        let fixture = Fixture::new();
        let path = fixture.package("chords-one", "@test/one");
        for contents in ["first", "edited"] {
            fs::write(path.join("chords/macos.toml"), contents).unwrap();
            let packages = import_monorepo_packages(&fixture.0).unwrap();
            assert_eq!(
                packages["@test/one"].chords_files_contents[Path::new("chords/macos.toml")],
                contents
            );
        }
        fixture.package("chords-two", "@test/two");
        assert_eq!(import_monorepo_packages(&fixture.0).unwrap().len(), 2);
        fs::remove_dir_all(path).unwrap();
        let packages = import_monorepo_packages(&fixture.0).unwrap();
        assert!(!packages.contains_key("@test/one"));
        assert!(packages.contains_key("@test/two"));
    }

    #[test]
    fn monorepo_validation_rejects_invalid_roots_and_duplicate_names() {
        let fixture = Fixture::new();
        for path in ["", "  ", fixture.0.to_str().unwrap()] {
            assert!(validate_monorepo_path(path).is_err());
        }
        fixture.package("chords-a", "@test/duplicate");
        fixture.package("chords-b", "@test/duplicate");
        assert!(
            import_monorepo_packages(&fixture.0)
                .unwrap_err()
                .to_string()
                .contains("Duplicate package")
        );
        fs::remove_dir_all(fixture.0.join("packages")).unwrap();
        assert!(discover_monorepo_packages(&fixture.0).is_err());
    }
}
