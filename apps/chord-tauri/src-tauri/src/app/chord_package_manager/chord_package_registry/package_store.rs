use crate::models::RawChordPackage;
use anyhow::{Context, Result, ensure};
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};

/// All package reads and worker roots go through this directory. Source registrations may
/// point elsewhere, but local sources are mounted here before even their manifests are read.
pub struct PackageStore {
    pub root: PathBuf,
}

impl PackageStore {
    pub fn for_app(handle: &AppHandle) -> Result<Self> {
        Ok(Self::new(handle.path().home_dir()?.join(".chord/packages")))
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn github_dir(&self) -> PathBuf {
        self.root.join(".sources/github.com")
    }

    pub fn is_official_root(&self, path: &Path) -> bool {
        self.contains(path) && path.strip_prefix(self.root.join("@keychord"))
            .is_ok_and(|relative| relative.components().next().is_some())
    }

    /// Only the official repository loader calls this. A manifest claiming an official
    /// name cannot put itself in the trusted namespace. Keep source/revision identities
    /// separate so pinned downloads and development checkouts never redirect each other.
    pub(super) fn mount_official(&self, source: &Path) -> Result<PathBuf> {
        let source = fs::canonicalize(source)?;
        let name = source.file_name().context("Package has no folder name")?;
        let link = self.root.join("@keychord/.managed").join(source_id(&source)).join(name);
        ensure_link(&source, &link)?;
        Ok(link)
    }

    pub fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
            && !path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
    }

    /// Preserve the original basename for packages without a package.json name. The full
    /// canonical source path scopes links, so two checkouts with the same basename coexist.
    pub fn mount(&self, source: &Path) -> Result<PathBuf> {
        if self.contains(source) {
            return Ok(source.to_path_buf());
        }
        let source = fs::canonicalize(source)
            .with_context(|| format!("Cannot access package source {}", source.display()))?;
        ensure!(
            source.is_dir(),
            "{} is not a package folder",
            source.display()
        );
        let name = source
            .file_name()
            .context("A package source must have a folder name")?;
        let link = self
            .root
            .join(".linked")
            .join(source_id(&source))
            .join(name);
        ensure_link(&source, &link)?;
        Ok(link)
    }

    pub fn import_package(&self, source: &Path) -> Result<RawChordPackage> {
        let root = self.mount(source)?;
        self.read_package(&root)
    }

    fn read_package(&self, root: &Path) -> Result<RawChordPackage> {
        ensure!(
            self.contains(root),
            "Packages must be read from {}",
            self.root.display()
        );
        ensure!(
            root.is_dir(),
            "Package folder {} is unavailable",
            root.display()
        );
        super::local::read_package(root)
    }

    /// Migrate old config packages without treating them as personal overrides.
    pub fn migrate_config(&self, old_root: &Path) -> Result<PathBuf> {
        let destination = self.root.join(".sources/config").join(source_id(old_root));
        if old_root.is_dir() {
            for entry in fs::read_dir(old_root)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    migrate_directory(&entry.path(), &destination.join(entry.file_name()))?;
                }
            }
        }
        Ok(destination)
    }
}

fn source_id(source: &Path) -> String {
    // A fixed algorithm keeps migrated config scopes discoverable across Rust upgrades.
    // DefaultHasher deliberately makes no such stability guarantee.
    let hash = source
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("{hash:016x}")
}

/// Reuse links only for this exact target. Never overwrite an existing directory
/// or silently redirect a link whose source identity conflicts with the requested source.
pub(super) fn ensure_link(source: &Path, link: &Path) -> Result<()> {
    match fs::symlink_metadata(link) {
        Ok(metadata) => {
            ensure!(
                metadata.file_type().is_symlink(),
                "{} is not a symlink",
                link.display()
            );
            ensure!(
                fs::read_link(link)? == source,
                "Conflicting package link at {}",
                link.display()
            );
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    fs::create_dir_all(link.parent().context("Package link has no parent")?)?;
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(source, link);
    #[cfg(windows)]
    let result = if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, link)
    } else {
        std::os::windows::fs::symlink_file(source, link)
    };
    match result {
        Ok(()) => Ok(()),
        // Another reload may have mounted the same source while this one was preparing it.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            ensure!(
                fs::read_link(link)? == source,
                "Conflicting package link at {}",
                link.display()
            );
            Ok(())
        }
        Err(error) => Err(error).with_context(|| format!("Cannot link {}", link.display())),
    }
}

/// Keep an existing destination intact. Cross-volume migrations stage a complete copy before
/// publishing it; a failed copy leaves the original checkout and its stored path untouched.
pub(super) fn migrate_directory(source: &Path, destination: &Path) -> Result<()> {
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        // A relative root link must keep pointing to the same checkout after changing parent.
        let target = fs::canonicalize(source)?;
        ensure_link(&target, destination)?;
        if let Err(error) = fs::remove_file(source) {
            log::warn!(
                "Package link migrated, but could not remove {}: {error}",
                source.display()
            );
        }
        return Ok(());
    }
    match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => anyhow::bail!(
            "Package destination {} already exists",
            destination.display()
        ),
    }
    let parent = destination
        .parent()
        .context("Package destination has no parent")?;
    fs::create_dir_all(parent)?;
    match fs::rename(source, destination) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::CrossesDevices => {}
        Err(error) => {
            return Err(error).context("Failed to migrate package into ~/.chord/packages");
        }
    }
    let temporary = parent.join(format!(".migrate-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        if fs::symlink_metadata(source)?.file_type().is_symlink() {
            ensure_link(&fs::read_link(source)?, &temporary)?;
        } else {
            copy_directory(source, &temporary)?;
        }
        fs::rename(&temporary, destination)?;
        Ok::<_, anyhow::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result?;
    // The destination is now authoritative. Retain the old copy if cleanup is unavailable.
    if let Err(error) = fs::remove_dir_all(source) {
        log::warn!(
            "Package migrated, but could not remove {}: {error}",
            source.display()
        );
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            ensure_link(&fs::read_link(entry.path())?, &target)?;
        } else if kind.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            anyhow::bail!("Unsupported package entry {}", entry.path().display());
        }
    }
    fs::set_permissions(destination, fs::metadata(source)?.permissions())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("chord-package-store-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&root).unwrap();
            Self(fs::canonicalize(root).unwrap())
        }
        fn store(&self) -> PackageStore {
            PackageStore::new(self.0.join("packages"))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn local_checkouts_are_scoped_and_only_read_through_package_links() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let first = fixture.0.join("one/chords-app");
        let second = fixture.0.join("two/chords-app");
        for source in [&first, &second] {
            fs::create_dir_all(source.join("chords")).unwrap();
            fs::write(source.join("chords/macos.toml"), "first").unwrap();
        }
        assert!(store.read_package(&first).is_err());
        let first_package = store.import_package(&first).unwrap();
        let second_package = store.import_package(&second).unwrap();
        assert!(store.contains(&first_package.root));
        assert_ne!(first_package.root, second_package.root);
        assert_eq!(first_package.dirname, "chords-app");
        let module = first.join("js/@test/vendor/js/handler.js");
        let relative = crate::app::chord_package_manager::package_module_relative_path(
            &first_package.root,
            &module,
        )
        .unwrap();
        assert_eq!(relative, Path::new("js/@test/vendor/js/handler.js"));
        let native = crate::app::chord_package_manager::resolve_logical_package_path(
            &relative,
            Path::new("target/test.node"),
        )
        .unwrap();
        assert_eq!(
            first_package.root.join(native),
            first_package
                .root
                .join("target/@test/vendor/target/test.node")
        );
        fs::write(first.join("chords/macos.toml"), "edited").unwrap();
        assert_eq!(
            store.import_package(&first).unwrap().chords_files_contents
                [Path::new("chords/macos.toml")],
            "edited"
        );
        assert!(
            store
                .import_package(&store.root.join("../one/chords-app"))
                .is_ok()
        );
        fs::remove_file(first_package.root).unwrap();
        assert!(first.is_dir());
    }

    #[test]
    fn conflicting_mounts_are_never_overwritten() {
        let fixture = Fixture::new();
        let source = fixture.0.join("checkout");
        let other = fixture.0.join("other");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&other).unwrap();
        let link = fixture.store().mount(&source).unwrap();
        fs::remove_file(&link).unwrap();
        ensure_link(&other, &link).unwrap();
        assert!(fixture.store().import_package(&source).is_err());
        assert_eq!(fs::read_link(link).unwrap(), other);
    }

    #[test]
    fn migration_preserves_contents_permissions_and_symlinks() {
        let fixture = Fixture::new();
        let source = fixture.0.join("legacy");
        let destination = fixture.store().root.join(".sources/config/legacy");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("handler.js"), "contents").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(source.join("handler.js"), fs::Permissions::from_mode(0o751))
                .unwrap();
        }
        let permissions = fs::metadata(source.join("handler.js"))
            .unwrap()
            .permissions();
        ensure_link(Path::new("handler.js"), &source.join("linked.js")).unwrap();
        // Exercise the cross-volume copy implementation without requiring a second volume.
        let copy = fixture.0.join("copy");
        copy_directory(&source, &copy).unwrap();
        assert_eq!(
            fs::read_link(copy.join("linked.js")).unwrap(),
            Path::new("handler.js")
        );
        assert_eq!(
            fs::metadata(copy.join("handler.js")).unwrap().permissions(),
            permissions
        );
        migrate_directory(&source, &destination).unwrap();
        assert!(!source.exists());
        assert_eq!(
            fs::read_to_string(destination.join("linked.js")).unwrap(),
            "contents"
        );
        assert!(migrate_directory(&copy, &destination).is_err());
        assert!(copy.exists());
    }

    #[test]
    fn migrating_a_relative_root_symlink_preserves_its_target() {
        let fixture = Fixture::new();
        let source = fixture.0.join("old-config/link");
        let checkout = fixture.0.join("checkout");
        fs::create_dir(&checkout).unwrap();
        fs::write(checkout.join("package.json"), r#"{"name":"test-linked"}"#).unwrap();
        ensure_link(Path::new("../checkout"), &source).unwrap();
        let root = fixture
            .store()
            .migrate_config(&fixture.0.join("old-config"))
            .unwrap();
        let package = fixture.store().import_package(&root.join("link")).unwrap();
        assert_eq!(package.package_name(), "test-linked");
        assert_eq!(fs::canonicalize(&package.root).unwrap(), checkout);
        assert!(!source.exists());
        assert!(checkout.exists());
    }

    #[test]
    fn legacy_config_packages_are_scoped_by_their_original_app_directory() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let mut migrated = Vec::new();
        for app in ["development", "production"] {
            let old = fixture.0.join(app);
            fs::create_dir_all(old.join("same-name/chords")).unwrap();
            fs::write(old.join("same-name/chords/macos.toml"), app).unwrap();
            let destination = store.migrate_config(&old).unwrap();
            let package = store
                .import_package(&destination.join("same-name"))
                .unwrap();
            assert_eq!(
                package.chords_files_contents[Path::new("chords/macos.toml")],
                app
            );
            migrated.push(package.root);
        }
        assert_ne!(migrated[0], migrated[1]);
    }
}
