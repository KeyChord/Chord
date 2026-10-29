use super::PackageStore;
use crate::models::RawChordPackage;
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find chords-* packages below the root, without descending into package contents.
pub fn discover_monorepo_packages(root: &Path) -> Result<Vec<PathBuf>> {
    ensure!(root.is_dir(), "{} is not a monorepo folder", root.display());
    let mut paths = Vec::new();
    let mut entries = WalkDir::new(root).min_depth(1).into_iter();
    while let Some(entry) = entries.next() {
        let entry = entry.with_context(|| format!("Cannot read monorepo {}", root.display()))?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if name.starts_with('.') || matches!(name.as_ref(), "node_modules" | "target") {
            entries.skip_current_dir();
        } else if name.starts_with("chords-") && is_package_root(entry.path()) {
            paths.push(entry.into_path());
            // Vendored packages belong to this package, not to the monorepo.
            entries.skip_current_dir();
        }
    }
    paths.sort();
    Ok(paths)
}

fn is_package_root(path: &Path) -> bool {
    path.join("package.json").is_file()
        || ["chords", "js", "bin"]
            .iter()
            .any(|dir| path.join(dir).is_dir())
}

pub fn validate_monorepo_path(store: &PackageStore, path: &str) -> Result<PathBuf> {
    ensure!(!path.trim().is_empty(), "Monorepo path cannot be empty");
    let root = fs::canonicalize(path.trim()).context("Failed to access monorepo folder")?;
    let mounted = store.mount(&root)?;
    ensure!(
        !discover_monorepo_packages(&mounted)?.is_empty(),
        "No chord packages found. Expected chords-* folders at the root or nested within {}",
        root.display()
    );
    // Detect unreadable packages and duplicate names before saving the source.
    import_monorepo_packages(store, &mounted)?;
    Ok(root)
}

pub fn import_monorepo_packages(
    store: &PackageStore,
    root: &Path,
) -> Result<HashMap<String, RawChordPackage>> {
    let root = store.mount(root)?;
    let mut packages: HashMap<String, RawChordPackage> = HashMap::new();
    for path in discover_monorepo_packages(&root)? {
        let package = store.import_package(&path)?;
        let name = package.package_name();
        if let Some(existing) = packages.get(&name) {
            anyhow::bail!(
                "Duplicate package {name} in monorepo {}: {} and {}",
                root.display(),
                existing.root.display(),
                path.display()
            );
        }
        packages.insert(name, package);
    }
    Ok(packages)
}

/// Missing selections deliberately mean no packages, including for older saved sources.
pub fn import_selected_monorepo_packages(
    store: &PackageStore,
    root: &Path,
    selected: &[String],
) -> Result<HashMap<String, RawChordPackage>> {
    if selected.is_empty() {
        return Ok(HashMap::new());
    }
    let mut packages = import_monorepo_packages(store, root)?;
    packages.retain(|name, _| selected.contains(name));
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
            fs::create_dir_all(&root).unwrap();
            Self(fs::canonicalize(root).unwrap())
        }
        fn store(&self) -> PackageStore {
            PackageStore::new(self.0.join(".store"))
        }
        fn package(&self, folder: &str, name: &str) -> PathBuf {
            let path = self.0.join(folder);
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
    fn monorepo_discovers_matching_directories_at_the_root_and_nested() {
        let fixture = Fixture::new();
        let first = fixture.package("chords-a", "@test/a");
        let last = fixture.package("chords-z", "@test/z");
        fixture.package("utility", "@test/ignored");
        let nested = fixture.package("group/deeper/chords-nested", "@test/nested");
        let legacy = fixture.package("packages/chords-legacy", "@test/legacy");
        fs::write(fixture.0.join("chords-file"), "not a folder").unwrap();
        assert_eq!(
            discover_monorepo_packages(&fixture.0).unwrap(),
            vec![first.clone(), last, nested, legacy]
        );
        let packages = import_monorepo_packages(&fixture.store(), &fixture.0).unwrap();
        assert_eq!(packages.len(), 4);
        assert_eq!(fs::canonicalize(&packages["@test/a"].root).unwrap(), first);
        assert!(validate_monorepo_path(&fixture.store(), fixture.0.to_str().unwrap()).is_ok());
    }

    #[test]
    fn monorepo_ignores_vendored_packages_dependencies_and_build_outputs() {
        let fixture = Fixture::new();
        let package = fixture.package("chords-one", "@test/one");
        for folder in [
            "chords-one/chords/@test/chords-vendored",
            "node_modules/@test/chords-dependency",
            "group/node_modules/chords-dependency",
            "target/chords-build",
            ".git/chords-metadata",
            ".jj/chords-metadata",
            ".moon/cache/states/chords-one",
            "group/.build/checkouts/chords-one",
            "node_modules/.cache/chord/native/chords-one",
        ] {
            fixture.package(folder, "@test/one");
        }
        assert_eq!(
            discover_monorepo_packages(&fixture.0).unwrap(),
            vec![package]
        );
        assert_eq!(
            import_monorepo_packages(&fixture.store(), &fixture.0)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn monorepo_ignores_leftover_folders_after_packages_move() {
        let fixture = Fixture::new();
        let package = fixture.package("@test/chords-one", "@test/one");
        fs::create_dir_all(fixture.0.join("packages/chords-one/.build/index-build")).unwrap();
        fs::create_dir_all(fixture.0.join("chords-empty")).unwrap();
        fs::create_dir_all(fixture.0.join(".moon/cache/states/chords-one")).unwrap();
        assert_eq!(
            discover_monorepo_packages(&fixture.0).unwrap(),
            vec![package.clone()]
        );
        let packages = import_monorepo_packages(&fixture.store(), &fixture.0).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(
            fs::canonicalize(&packages["@test/one"].root).unwrap(),
            package
        );
        assert!(validate_monorepo_path(&fixture.store(), fixture.0.to_str().unwrap()).is_ok());
    }

    #[test]
    fn monorepo_supports_manifest_only_and_manifestless_packages() {
        let fixture = Fixture::new();
        let manifest = fixture.package("chords-manifest", "@test/manifest");
        fs::remove_dir(manifest.join("chords")).unwrap();
        for dir in ["chords", "js", "bin"] {
            fs::create_dir_all(fixture.0.join(format!("chords-{dir}/{dir}"))).unwrap();
        }
        // A matching container without package contents must not hide nested packages.
        let nested = fixture.package("chords-container/group/chords-nested", "@test/nested");
        let packages = import_monorepo_packages(&fixture.store(), &fixture.0).unwrap();
        assert_eq!(packages.len(), 5);
        assert_eq!(
            fs::canonicalize(&packages["@test/manifest"].root).unwrap(),
            manifest
        );
        assert_eq!(
            fs::canonicalize(&packages["@test/nested"].root).unwrap(),
            nested
        );
        for dir in ["chords", "js", "bin"] {
            let name = format!("chords-{dir}");
            assert_eq!(
                fs::canonicalize(&packages[&name].root).unwrap(),
                fixture.0.join(&name)
            );
        }
    }

    #[test]
    fn monorepo_reload_picks_up_edits_additions_and_removals() {
        let fixture = Fixture::new();
        let path = fixture.package("chords-one", "@test/one");
        for contents in ["first", "edited"] {
            fs::write(path.join("chords/macos.toml"), contents).unwrap();
            let packages = import_monorepo_packages(&fixture.store(), &fixture.0).unwrap();
            assert_eq!(
                packages["@test/one"].chords_files_contents[Path::new("chords/macos.toml")],
                contents
            );
        }
        fixture.package("chords-two", "@test/two");
        assert_eq!(
            import_monorepo_packages(&fixture.store(), &fixture.0)
                .unwrap()
                .len(),
            2
        );
        fs::remove_dir_all(path).unwrap();
        let packages = import_monorepo_packages(&fixture.store(), &fixture.0).unwrap();
        assert!(!packages.contains_key("@test/one"));
        assert!(packages.contains_key("@test/two"));
    }

    #[test]
    fn monorepo_requires_selection_and_does_not_enable_new_packages() {
        let fixture = Fixture::new();
        let path = fixture.package("chords-one", "@test/one");
        assert!(
            import_selected_monorepo_packages(&fixture.store(), &fixture.0, &[])
                .unwrap()
                .is_empty()
        );
        let selected = vec!["@test/one".to_owned()];
        assert_eq!(
            import_selected_monorepo_packages(&fixture.store(), &fixture.0, &selected)
                .unwrap()
                .len(),
            1
        );
        fixture.package("chords-two", "@test/two");
        fs::write(path.join("chords/macos.toml"), "edited").unwrap();
        let packages =
            import_selected_monorepo_packages(&fixture.store(), &fixture.0, &selected).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(
            packages["@test/one"].chords_files_contents[Path::new("chords/macos.toml")],
            "edited"
        );
        fs::remove_dir_all(path).unwrap();
        assert!(
            import_selected_monorepo_packages(&fixture.store(), &fixture.0, &selected)
                .unwrap()
                .is_empty()
        );
        assert!(
            import_selected_monorepo_packages(&fixture.store(), &fixture.0, &[])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn monorepo_validation_rejects_invalid_roots_and_duplicate_names() {
        let fixture = Fixture::new();
        for path in ["", "  ", fixture.0.to_str().unwrap()] {
            assert!(validate_monorepo_path(&fixture.store(), path).is_err());
        }
        fixture.package("chords-a", "@test/duplicate");
        fixture.package("chords-b", "@test/duplicate");
        assert!(
            import_monorepo_packages(&fixture.store(), &fixture.0)
                .unwrap_err()
                .to_string()
                .contains("Duplicate package")
        );
        fs::remove_dir_all(&fixture.0).unwrap();
        assert!(discover_monorepo_packages(&fixture.0).is_err());
    }
}
