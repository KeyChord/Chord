use super::PackageStore;
use crate::models::RawChordPackage;
use anyhow::{Context, Result, ensure};
use nject::{inject, injectable};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use typeshare::typeshare;

const STORE: &str = "package-trust.json";
const APPROVALS: &str = "approvedPackages";

/// Approval belongs to a location, never to a package's self-reported name.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyPackage {
    pub name: String,
    pub path: String,
    /// Includes the resolved target so replacing a symlink requires approval again.
    pub identity: String,
    pub approved: bool,
}

pub(super) fn package_identity(root: &Path) -> Result<String> {
    Ok(serde_json::to_string(&(
        root,
        std::fs::canonicalize(root)
            .with_context(|| format!("Package {} is unavailable", root.display()))?,
    ))?)
}

#[injectable]
pub struct PackageTrustRegistry {
    handle: AppHandle,
    #[inject(parking_lot::Mutex::new(()))]
    write_lock: parking_lot::Mutex<()>,
}

impl PackageTrustRegistry {
    pub fn approvals(&self) -> Result<BTreeSet<String>> {
        let _guard = self.write_lock.lock();
        self.read_approvals()
    }

    fn read_approvals(&self) -> Result<BTreeSet<String>> {
        self.handle
            .store(STORE)?
            .get(APPROVALS)
            .map(serde_json::from_value)
            .transpose()
            .context("Failed to read package approvals")
            .map(Option::unwrap_or_default)
    }

    /// Applies every change in one store write, so a batch is approved or revoked as a whole.
    pub fn set_approvals(&self, packages: &[(&Path, &str)], approved: bool) -> Result<()> {
        let _guard = self.write_lock.lock();
        if approved {
            for (root, identity) in packages {
                ensure!(
                    package_identity(root)? == *identity,
                    "Package {} changed location. Reload Chord and review it again.",
                    root.display()
                );
            }
        }
        let store = self.handle.store(STORE)?;
        let previous = store.get(APPROVALS);
        let mut approvals = self.read_approvals()?;
        for (_, identity) in packages {
            if approved {
                approvals.insert((*identity).to_owned());
            } else {
                approvals.remove(*identity);
            }
        }
        store.set(APPROVALS, serde_json::to_value(approvals)?);
        if let Err(error) = store.save() {
            match previous {
                Some(value) => store.set(APPROVALS, value),
                None => {
                    store.delete(APPROVALS);
                }
            }
            return Err(error.into());
        }
        Ok(())
    }

    /// Only packages Chord has just created are approved implicitly. Existing folders,
    /// including an existing `default`, always go through the normal trust check.
    pub fn approve_created_package(&self, root: &Path) -> Result<()> {
        self.set_approvals(&[(root, &package_identity(root)?)], true)
    }
}

/// Applied to each source tier before merging, so a blocked same-name package cannot
/// suppress an already trusted package. No handler compilation happens before this gate.
pub(super) fn filter_packages(
    store: &PackageStore,
    packages: &mut HashMap<String, RawChordPackage>,
    approvals: &BTreeSet<String>,
) -> Result<Vec<ThirdPartyPackage>> {
    let mut third_party = Vec::new();
    let mut blocked = Vec::new();
    for (name, package) in packages.iter() {
        if store.is_official_root(&package.root) {
            continue;
        }
        let identity = package_identity(&package.root)?;
        let approved = approvals.contains(&identity);
        third_party.push(ThirdPartyPackage {
            name: name.clone(),
            path: package.root.display().to_string(),
            identity,
            approved,
        });
        if !approved {
            blocked.push(name.clone());
        }
    }
    for name in blocked {
        packages.remove(&name);
    }
    Ok(third_party)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Fixture(PackageStore);
    impl Fixture {
        fn new() -> Self {
            Self(PackageStore::new(
                std::env::temp_dir().join(format!("chord-trust-{}", uuid::Uuid::new_v4())),
            ))
        }
        fn package(&self, location: &str, name: &str) -> HashMap<String, RawChordPackage> {
            let root = self.0.root.join(location);
            fs::create_dir_all(root.join("chords")).unwrap();
            fs::create_dir_all(root.join("js")).unwrap();
            fs::write(
                root.join("package.json"),
                serde_json::json!({"name": name}).to_string(),
            )
            .unwrap();
            fs::write(
                root.join("chords/macos.toml"),
                "name = 'Test'\n[chords]\n'/t' = { shell = 'true' }",
            )
            .unwrap();
            fs::write(
                root.join("js/handler.js"),
                "throw new Error('must not import before approval');",
            )
            .unwrap();
            HashMap::from([(name.to_owned(), self.0.import_package(&root).unwrap())])
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }

    #[test]
    fn every_source_is_blocked_by_default_including_spoofed_official_and_core_names() {
        let fixture = Fixture::new();
        for location in [
            "downloaded",
            "default",
            "@someone/chords-test",
            ".sources/github.com/other/repo/revision",
            ".linked/checkout/chords-test",
        ] {
            for name in [
                "third-party",
                "@keychord/chords-test",
                crate::models::CORE_CHORD_PACKAGE,
            ] {
                let mut packages = fixture.package(location, name);
                let blocked = filter_packages(&fixture.0, &mut packages, &BTreeSet::new()).unwrap();
                assert!(
                    packages.is_empty(),
                    "{location}: {name} must not reach compilation"
                );
                assert_eq!(blocked.len(), 1);
                assert!(!blocked[0].approved);
            }
        }
    }

    #[test]
    fn only_the_official_directory_bypasses_approval() {
        let fixture = Fixture::new();
        let mut official = fixture.package("@keychord/chords-test", "@keychord/chords-test");
        assert!(
            filter_packages(&fixture.0, &mut official, &BTreeSet::new())
                .unwrap()
                .is_empty()
        );
        assert_eq!(official.len(), 1);
        for path in [
            "@keychord",
            "@keychord-fake/chords-test",
            "other/@keychord/chords-test",
            "@keychord/../evil",
        ] {
            assert!(
                !fixture.0.is_official_root(&fixture.0.root.join(path)),
                "{path}"
            );
        }
        assert!(
            !fixture
                .0
                .is_official_root(Path::new("/elsewhere/@keychord/chords-test"))
        );
    }

    #[test]
    fn approval_survives_reloading_but_does_not_apply_to_another_same_named_package() {
        let fixture = Fixture::new();
        let mut packages = fixture.package("downloaded", "test");
        let pending = filter_packages(&fixture.0, &mut packages, &BTreeSet::new())
            .unwrap()
            .remove(0);
        let saved = serde_json::to_string(&BTreeSet::from([pending.identity.clone()])).unwrap();
        let approvals = serde_json::from_str(&saved).unwrap();
        let mut reloaded = fixture.package("downloaded", "test");
        let state = filter_packages(&fixture.0, &mut reloaded, &approvals).unwrap();
        assert!(state[0].approved);
        assert_eq!(reloaded.len(), 1);
        let mut other = fixture.package("another", "test");
        assert!(!filter_packages(&fixture.0, &mut other, &approvals).unwrap()[0].approved);
        assert!(other.is_empty());
        assert!(!filter_packages(&fixture.0, &mut reloaded, &BTreeSet::new()).unwrap()[0].approved);
        assert!(reloaded.is_empty());
    }

    #[test]
    fn a_blocked_override_keeps_the_trusted_package_and_core_controls_available() {
        let fixture = Fixture::new();
        let name = crate::models::CORE_CHORD_PACKAGE;
        let mut trusted = fixture.package("@keychord/chords-chord", name);
        let mut untrusted = fixture.package("override", name);
        filter_packages(&fixture.0, &mut untrusted, &BTreeSet::new()).unwrap();
        trusted.extend(untrusted);
        super::super::core::protect_core_package(&mut trusted, &fixture.0).unwrap();
        assert_eq!(
            trusted[name].root,
            fixture.0.root.join("@keychord/chords-chord")
        );
        assert!(
            trusted[name]
                .chords_files_contents
                .contains_key(Path::new(crate::models::CORE_CHORD_FILE))
        );
    }

    #[cfg(unix)]
    #[test]
    fn retargeting_a_link_does_not_inherit_approval_and_local_official_names_are_untrusted() {
        let fixture = Fixture::new();
        let first = fixture
            .package("first", "@keychord/test")
            .remove("@keychord/test")
            .unwrap();
        let second = fixture
            .package("second", "@keychord/test")
            .remove("@keychord/test")
            .unwrap();
        let link = fixture.0.root.join("linked");
        std::os::unix::fs::symlink(&first.root, &link).unwrap();
        let identity = package_identity(&link).unwrap();
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&second.root, &link).unwrap();
        assert_ne!(package_identity(&link).unwrap(), identity);
        let mut packages = HashMap::from([(
            "@keychord/test".into(),
            fixture.0.import_package(&link).unwrap(),
        )]);
        assert!(
            !filter_packages(&fixture.0, &mut packages, &BTreeSet::from([identity])).unwrap()[0]
                .approved
        );
        assert!(packages.is_empty());
        // Only the verified official-download path can create the trusted managed alias.
        let official = fixture.0.mount_official(&first.root).unwrap();
        assert!(fixture.0.is_official_root(&official));
        assert_eq!(
            fs::canonicalize(official).unwrap(),
            fs::canonicalize(first.root).unwrap()
        );
    }
}
