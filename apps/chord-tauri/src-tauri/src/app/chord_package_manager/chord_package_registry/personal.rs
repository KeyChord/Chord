use super::{LocalPackageRegistry, is_hidden_entry};
use crate::models::{ParsedChordsFile, RawChordPackage, RawChordsFile};
use anyhow::Context;
use nject::injectable;
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};
use typeshare::typeshare;
use walkdir::WalkDir;

/// Chord creates this package when it is missing, so there is always a place for new chords.
pub const DEFAULT_PERSONAL_PACKAGE: &str = "default";

/// The file every new personal package starts with.
const STARTER_PATHSLUG: &str = "chords/macos.toml";

fn starter_contents(dirname: &str) -> String {
    format!(
        r#"# Your personal chords. When a chord here uses the same keys as a chord from
# another package, this one runs.
#
# Chords that start with a symbol work in every app, for example:
#   "/q" = {{ name = "Force Quit", shortcut = "cmd+opt+esc" }}
#   "/t" = {{ name = "Terminal", shell = "open -a Terminal" }}
#
# Chords for a single app go in chords/<bundle ID, with / for .>/macos.toml,
# such as chords/com/apple/finder/macos.toml for Finder.
name = "{dirname}"

[chords]
"#
    )
}

/// One folder in `~/.chord/packages`, as the last reload saw it.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalPackageState {
    pub dirname: String,
    /// The name the package loaded under: its `package.json` name, else the folder name.
    pub name: Option<String>,
    /// Why the folder could not be loaded at all. Problems in single files are reported with
    /// the other packages' load errors, under `name`.
    pub error: Option<String>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PersonalPackages {
    /// `~/.chord/packages`.
    pub root: String,
    /// Package folders, sorted by name, which is also their priority order.
    pub packages: Vec<PersonalPackageFiles>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PersonalPackageFiles {
    pub dirname: String,
    pub path: String,
    /// Chords files relative to the package root (`chords/…`), sorted.
    pub files: Vec<String>,
}

/// A problem that keeps a chords file from loading. Offsets count UTF-16 code units, as
/// JavaScript strings do, and are present when the problem has a location in the file.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileProblem {
    pub message: String,
    pub from: Option<u32>,
    pub to: Option<u32>,
}

/// The user's own chord packages: every folder in `~/.chord/packages`. They load after every
/// other source, so they override packages of the same name, and their chords take precedence
/// in each scope.
#[injectable]
pub struct PersonalPackageRegistry {
    handle: AppHandle,
}

impl PersonalPackageRegistry {
    pub fn packages_dir(&self) -> anyhow::Result<PathBuf> {
        Ok(self.handle.path().home_dir()?.join(".chord").join("packages"))
    }

    pub fn ensure_default_package(&self) -> anyhow::Result<()> {
        ensure_default_package(&self.packages_dir()?)
    }

    pub fn import_packages(
        &self,
    ) -> anyhow::Result<(HashMap<String, RawChordPackage>, Vec<PersonalPackageState>)> {
        Ok(import_packages(&self.packages_dir()?))
    }

    pub fn list(&self) -> anyhow::Result<PersonalPackages> {
        list(&self.packages_dir()?)
    }

    pub fn package_root(&self, dirname: &str) -> anyhow::Result<PathBuf> {
        package_root(&self.packages_dir()?, dirname)
    }

    pub fn create_package(&self, dirname: &str) -> anyhow::Result<()> {
        create_package(&self.packages_dir()?, dirname)
    }

    pub fn read_file(&self, dirname: &str, pathslug: &str) -> anyhow::Result<String> {
        read_file(&self.package_root(dirname)?, pathslug)
    }

    pub fn save_file(
        &self,
        dirname: &str,
        pathslug: &str,
        contents: &str,
        expected: Option<&str>,
    ) -> anyhow::Result<()> {
        save_file(&self.package_root(dirname)?, pathslug, contents, expected)
    }

    pub fn delete_file(&self, dirname: &str, pathslug: &str) -> anyhow::Result<()> {
        delete_file(&self.package_root(dirname)?, pathslug)
    }
}

fn ensure_default_package(dir: &Path) -> anyhow::Result<()> {
    if dir.join(DEFAULT_PERSONAL_PACKAGE).exists() {
        return Ok(());
    }
    create_package(dir, DEFAULT_PERSONAL_PACKAGE)
}

/// Package folders in priority order: visible directories (or links to them), sorted by name.
fn package_dirnames(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut dirnames = Vec::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let dirname = entry.file_name().to_string_lossy().into_owned();
                if !dirname.starts_with('.') && entry.path().is_dir() {
                    dirnames.push(dirname);
                }
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {}", dir.display()));
        }
    }
    dirnames.sort();
    Ok(dirnames)
}

/// A folder that fails to import is reported and skipped; it never keeps the others from
/// loading. When two folders load under the same name, the first in priority order wins.
fn import_packages(dir: &Path) -> (HashMap<String, RawChordPackage>, Vec<PersonalPackageState>) {
    let mut packages = HashMap::new();
    let mut states = Vec::new();
    let dirnames = match package_dirnames(dir) {
        Ok(dirnames) => dirnames,
        Err(error) => {
            log::warn!("Error listing personal chord packages: {error:#}");
            return (packages, states);
        }
    };
    for dirname in dirnames {
        let (name, error) = match LocalPackageRegistry::import_from_local_folder(&dir.join(&dirname))
        {
            Ok(package) => {
                let name = package.package_name();
                if let Some(first) = states
                    .iter()
                    .find(|state: &&PersonalPackageState| state.name.as_ref() == Some(&name))
                {
                    let error = format!(
                        "{name} is already loaded from the {} folder; give this package a different name in its package.json",
                        first.dirname
                    );
                    (None, Some(error))
                } else {
                    packages.insert(name.clone(), package);
                    (Some(name), None)
                }
            }
            Err(error) => (None, Some(format!("{error:#}"))),
        };
        if let Some(error) = &error {
            log::warn!("Error importing personal chord package {dirname}: {error}, skipping");
        }
        states.push(PersonalPackageState {
            dirname,
            name,
            error,
        });
    }
    (packages, states)
}

fn list(dir: &Path) -> anyhow::Result<PersonalPackages> {
    let mut packages = Vec::new();
    for dirname in package_dirnames(dir)? {
        let root = dir.join(&dirname);
        packages.push(PersonalPackageFiles {
            files: chords_files(&root)?,
            path: root.display().to_string(),
            dirname,
        });
    }
    Ok(PersonalPackages {
        root: dir.display().to_string(),
        packages,
    })
}

/// Walks `chords/` as the loader does; only `.toml` files can be opened in the editor.
fn chords_files(root: &Path) -> anyhow::Result<Vec<String>> {
    let chords_dir = root.join("chords");
    let mut files = Vec::new();
    if !chords_dir.is_dir() {
        return Ok(files);
    }
    for entry in WalkDir::new(&chords_dir)
        .into_iter()
        .filter_entry(|entry| !is_hidden_entry(entry))
    {
        let path = entry?.into_path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "toml") {
            files.push(path.strip_prefix(root)?.to_string_lossy().into_owned());
        }
    }
    files.sort();
    Ok(files)
}

fn validate_dirname(dirname: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        dirname.len() <= 100
            && dirname
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_alphanumeric())
            && dirname
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
        "Package folder names start with a letter or digit and use only letters, digits, hyphens, underscores, and periods"
    );
    Ok(())
}

fn package_root(dir: &Path, dirname: &str) -> anyhow::Result<PathBuf> {
    validate_dirname(dirname)?;
    let root = dir.join(dirname);
    anyhow::ensure!(root.is_dir(), "{} does not exist", root.display());
    Ok(root)
}

fn create_package(dir: &Path, dirname: &str) -> anyhow::Result<()> {
    validate_dirname(dirname)?;
    fs::create_dir_all(dir).with_context(|| format!("Failed to create {}", dir.display()))?;
    let root = dir.join(dirname);
    // `create_dir` fails when the folder exists, so an existing package is never touched.
    fs::create_dir(&root).map_err(|error| match error.kind() {
        ErrorKind::AlreadyExists => anyhow::anyhow!("{} already exists", root.display()),
        _ => anyhow::Error::new(error).context(format!("Failed to create {}", root.display())),
    })?;
    save_file(&root, STARTER_PATHSLUG, &starter_contents(dirname), None)
}

fn read_file(root: &Path, pathslug: &str) -> anyhow::Result<String> {
    let path = chords_file_path(root, pathslug)?;
    fs::read_to_string(&path).with_context(|| format!("Failed to read {pathslug}"))
}

/// The same checks the loader makes before compiling a chords file.
fn check_contents(contents: &str) -> anyhow::Result<()> {
    toml::from_str::<RawChordsFile>(contents)?;
    contents.parse::<ParsedChordsFile>()?;
    Ok(())
}

pub fn chords_file_problem(contents: &str) -> Option<ChordsFileProblem> {
    let error = check_contents(contents).err()?;
    let toml_error = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<toml::de::Error>());
    let span = toml_error.and_then(toml::de::Error::span);
    let utf16 = |byte: usize| {
        let byte = (0..=byte.min(contents.len()))
            .rev()
            .find(|&index| contents.is_char_boundary(index))
            .unwrap_or(0);
        contents[..byte].encode_utf16().count() as u32
    };
    Some(ChordsFileProblem {
        // A located TOML error's display repeats the source excerpt the editor already shows.
        message: match (toml_error, &span) {
            (Some(toml_error), Some(_)) => toml_error.message().trim().to_owned(),
            _ => format!("{error:#}"),
        },
        from: span.as_ref().map(|span| utf16(span.start)),
        to: span.as_ref().map(|span| utf16(span.end)),
    })
}

/// Writes a chords file after checking that it would load. `expected` is the contents the
/// editor started from, or `None` to create a new file; a file that changed on disk since then
/// is not overwritten.
fn save_file(
    root: &Path,
    pathslug: &str,
    contents: &str,
    expected: Option<&str>,
) -> anyhow::Result<()> {
    let path = chords_file_path(root, pathslug)?;
    check_contents(contents).with_context(|| format!("Invalid {pathslug}"))?;

    let current = match fs::read_to_string(&path) {
        Ok(current) => Some(current),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {pathslug}"));
        }
    };
    match (expected, current.as_deref()) {
        (None, Some(_)) => anyhow::bail!("{pathslug} already exists"),
        (Some(_), None) => anyhow::bail!("{pathslug} was deleted since you opened it"),
        (Some(expected), Some(current)) if expected != current => anyhow::bail!(
            "{pathslug} changed on disk since you opened it. Revert to load the new version."
        ),
        _ => {}
    }

    let parent = path.parent().context("chords file has no parent folder")?;
    fs::create_dir_all(parent).with_context(|| format!("Failed to create {}", parent.display()))?;
    write_atomically(&path, contents).with_context(|| format!("Failed to write {pathslug}"))
}

/// Replaces the file in one step so a reload never sees it half-written. A symlinked file is
/// written at its target, so the link (for example into a dotfiles repository) is kept, and an
/// existing file keeps its permissions.
fn write_atomically(path: &Path, contents: &str) -> anyhow::Result<()> {
    let target = match fs::canonicalize(path) {
        Ok(target) => target,
        Err(error) if error.kind() == ErrorKind::NotFound => path.to_path_buf(),
        Err(error) => return Err(error.into()),
    };
    let permissions = fs::metadata(&target).ok().map(|metadata| metadata.permissions());
    let parent = target.parent().context("chords file has no parent folder")?;
    let file_name = target
        .file_name()
        .context("chords file has no name")?
        .to_string_lossy();
    // Hidden, so a reload during the write never reads it as a chords file.
    let temporary = parent.join(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        if let Some(permissions) = permissions {
            fs::set_permissions(&temporary, permissions)?;
        }
        fs::rename(&temporary, &target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    Ok(result?)
}

fn delete_file(root: &Path, pathslug: &str) -> anyhow::Result<()> {
    let path = chords_file_path(root, pathslug)?;
    fs::remove_file(&path).with_context(|| format!("Failed to delete {pathslug}"))?;
    // Remove folders the file leaves empty, such as `com/apple/finder/`, but keep `chords/`.
    let chords_dir = root.join("chords");
    for folder in path.ancestors().skip(1) {
        if folder == chords_dir || !folder.starts_with(&chords_dir) || fs::remove_dir(folder).is_err()
        {
            break;
        }
    }
    Ok(())
}

/// Resolves a `chords/…/*.toml` path inside the package, rejecting anything that could escape
/// it or that the loader would not read as a chords file.
fn chords_file_path(root: &Path, pathslug: &str) -> anyhow::Result<PathBuf> {
    let relative = Path::new(pathslug);
    let mut components = relative.components();
    anyhow::ensure!(
        components.next() == Some(Component::Normal("chords".as_ref()))
            && components.clone().next().is_some()
            && components.all(|component| matches!(component, Component::Normal(_))),
        "{pathslug} must be a path inside chords/"
    );
    // The loader skips hidden entries, so a file saved under one would never load.
    anyhow::ensure!(
        relative.extension().is_some_and(|extension| extension == "toml")
            && !relative
                .components()
                .any(|component| component.as_os_str().to_string_lossy().starts_with('.')),
        "{pathslug} must be a .toml file outside hidden folders, with a name that does not start with a dot"
    );
    Ok(root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_dir() -> PathBuf {
        std::env::temp_dir().join(format!("chord-personal-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn the_default_package_is_created_once_and_never_replaced() {
        let dir = temporary_dir();
        assert!(import_packages(&dir).1.is_empty());
        assert!(list(&dir).unwrap().packages.is_empty());

        ensure_default_package(&dir).unwrap();
        let (packages, states) = import_packages(&dir);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].name.as_deref(), Some(DEFAULT_PERSONAL_PACKAGE));
        assert!(packages[DEFAULT_PERSONAL_PACKAGE]
            .chords_files_contents
            .contains_key(Path::new(STARTER_PATHSLUG)));
        assert!(chords_file_problem(&starter_contents(DEFAULT_PERSONAL_PACKAGE)).is_none());

        let root = dir.join(DEFAULT_PERSONAL_PACKAGE);
        let starter = starter_contents(DEFAULT_PERSONAL_PACKAGE);
        let edited = "name = 'Mine'\n[chords]\n'/x' = { name = 'X', shell = 'true' }\n";
        save_file(&root, STARTER_PATHSLUG, edited, Some(&starter)).unwrap();
        ensure_default_package(&dir).unwrap();
        assert!(create_package(&dir, DEFAULT_PERSONAL_PACKAGE).is_err());
        assert_eq!(read_file(&root, STARTER_PATHSLUG).unwrap(), edited);

        // Deleting the starter file is the user's choice too.
        delete_file(&root, STARTER_PATHSLUG).unwrap();
        ensure_default_package(&dir).unwrap();
        assert!(list(&dir).unwrap().packages[0].files.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn every_visible_folder_loads_in_name_order_and_broken_ones_are_reported() {
        let dir = temporary_dir();
        for dirname in ["work", "default", "alpha"] {
            create_package(&dir, dirname).unwrap();
        }
        fs::create_dir_all(dir.join(".hidden/chords")).unwrap();
        fs::write(dir.join("notes.txt"), "not a package").unwrap();
        // Two folders claiming one name: the first in priority order keeps it.
        fs::write(dir.join("work/package.json"), r#"{"name":"alpha"}"#).unwrap();
        fs::create_dir_all(dir.join("broken/chords")).unwrap();
        fs::write(dir.join("broken/chords/macos.toml"), [0xff, 0xfe]).unwrap();

        let (packages, states) = import_packages(&dir);
        let dirnames: Vec<_> = states.iter().map(|state| state.dirname.as_str()).collect();
        assert_eq!(dirnames, ["alpha", "broken", "default", "work"]);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages["alpha"].root, dir.join("alpha"));
        assert!(states[1].name.is_none() && states[1].error.is_some());
        assert!(states[3].name.is_none());
        assert!(states[3].error.as_deref().unwrap().contains("alpha is already loaded from the alpha folder"));

        let listed = list(&dir).unwrap();
        assert_eq!(listed.packages.len(), 4);
        assert_eq!(listed.packages[2].files, vec![STARTER_PATHSLUG]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn package_and_file_paths_must_stay_inside_the_packages_folder() {
        let dir = temporary_dir();
        create_package(&dir, "default").unwrap();
        for dirname in ["", "..", ".hidden", "a/b", "-x", "with space", "missing"] {
            assert!(package_root(&dir, dirname).is_err(), "{dirname:?}");
        }
        for dirname in ["..", ".chord", "a/b"] {
            assert!(create_package(&dir, dirname).is_err(), "{dirname:?}");
        }
        let root = package_root(&dir, "default").unwrap();
        for pathslug in [
            "macos.toml",
            "chords",
            "chords/../macos.toml",
            "/chords/macos.toml",
            "js/macos.toml",
            "chords/macos.js",
            "chords/.hidden.toml",
            "chords/.git/macos.toml",
        ] {
            assert!(chords_file_path(&root, pathslug).is_err(), "{pathslug}");
        }
        assert_eq!(
            chords_file_path(&root, "chords/com/apple/finder/macos.toml").unwrap(),
            root.join("chords/com/apple/finder/macos.toml")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saving_checks_contents_and_refuses_to_overwrite_newer_changes() {
        let dir = temporary_dir();
        create_package(&dir, "default").unwrap();
        let root = dir.join("default");
        let app = "chords/com/apple/finder/macos.toml";
        let contents = "name = 'Finder'\n[chords]\ngd = { name = 'Downloads', shortcut = 'opt+cmd+l' }\n";
        save_file(&root, app, contents, None).unwrap();
        assert!(
            save_file(&root, app, contents, None)
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );

        for invalid in [
            "[chords",
            "[chords]\n",
            "name = 'x'\n[chords]\n'0x' = { name = 'x', shell = 'true' }",
        ] {
            assert!(save_file(&root, app, invalid, Some(contents)).is_err(), "{invalid}");
        }
        assert_eq!(read_file(&root, app).unwrap(), contents);

        let external = format!("{contents}# edited elsewhere\n");
        fs::write(root.join(app), &external).unwrap();
        let error = save_file(&root, app, contents, Some(contents)).unwrap_err();
        assert!(error.to_string().contains("changed on disk"), "{error}");
        assert_eq!(read_file(&root, app).unwrap(), external);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn problems_point_at_their_location_in_utf16_offsets() {
        // `é` is two UTF-8 bytes but one UTF-16 code unit.
        let contents = "name = 'é'\n[chords\n";
        let problem = chords_file_problem(contents).unwrap();
        let start = contents.find("[chords").unwrap() - 1;
        assert!(problem.from.unwrap() as usize >= start, "{:?}", problem.from);
        assert!(problem.to.unwrap() <= contents.encode_utf16().count() as u32);
        assert!(!problem.message.contains("TOML parse error at line"), "{}", problem.message);

        // Chord's own rules have no location in the file.
        let reserved = "name = 'x'\n[chords]\n'0x' = { name = 'x', shell = 'true' }\n";
        let problem = chords_file_problem(reserved).unwrap();
        assert!(problem.message.contains("0 prefix is reserved"), "{}", problem.message);
        assert!(problem.from.is_none());

        let missing_name = chords_file_problem("[chords]\n").unwrap();
        assert!(missing_name.message.contains("name"), "{}", missing_name.message);
    }

    #[cfg(unix)]
    #[test]
    fn saving_keeps_symlinks_and_permissions_and_deleting_prunes_empty_folders() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = temporary_dir();
        create_package(&dir, "default").unwrap();
        let root = dir.join("default");
        let dotfiles = temporary_dir();
        fs::create_dir_all(&dotfiles).unwrap();
        let target = dotfiles.join("macos.toml");
        let starter = starter_contents("default");
        fs::write(&target, &starter).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        fs::remove_file(root.join(STARTER_PATHSLUG)).unwrap();
        symlink(&target, root.join(STARTER_PATHSLUG)).unwrap();

        let edited = "name = 'Linked'\n[chords]\n";
        save_file(&root, STARTER_PATHSLUG, edited, Some(&starter)).unwrap();
        assert!(fs::symlink_metadata(root.join(STARTER_PATHSLUG)).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), edited);
        assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o600);

        let app = "chords/com/apple/finder/macos.toml";
        save_file(&root, app, edited, None).unwrap();
        // Finder leaves `.DS_Store` files in folders the user opens.
        fs::write(root.join("chords/.DS_Store"), [0, 0, 0, 1, 0xff, 0xfe]).unwrap();
        assert_eq!(chords_files(&root).unwrap(), vec![app, STARTER_PATHSLUG]);
        assert!(import_packages(&dir).1[0].error.is_none());
        delete_file(&root, app).unwrap();
        assert!(!root.join("chords/com").exists());
        assert!(root.join("chords").is_dir());
        fs::remove_dir_all(dir).unwrap();
        fs::remove_dir_all(dotfiles).unwrap();
    }
}
