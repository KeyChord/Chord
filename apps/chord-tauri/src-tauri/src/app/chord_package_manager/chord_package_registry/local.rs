use crate::app::AppHandleExt;
use crate::app::state::AppSingleton;
use crate::models::RawChordPackage;
use anyhow::Context;
use nject::injectable;
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::{Store, StoreExt};
use walkdir::WalkDir;

pub const CHORD_SOURCES_STORE_PATH: &str = "chord-sources.json";
pub const LOCAL_FOLDERS_KEY: &str = "localFolders";
pub const LOCAL_MONOREPOS_KEY: &str = "localMonorepos";

const MAX_SOURCE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PACKAGE_SOURCE_BYTES: u64 = 32 * 1024 * 1024;

pub(super) fn read_package_source(path: &Path, remaining: &mut u64) -> anyhow::Result<String> {
    anyhow::ensure!(
        fs::metadata(path)?.is_file(),
        "package source {} must be a regular file",
        path.display()
    );
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    let limit = MAX_SOURCE_BYTES.min(*remaining);
    anyhow::ensure!(
        metadata.is_file() && metadata.len() <= limit,
        "package source {} exceeds its size budget (8 MiB per file, 32 MiB per package)",
        path.display()
    );
    let mut contents = Vec::new();
    file.take(limit + 1).read_to_end(&mut contents)?;
    anyhow::ensure!(
        contents.len() as u64 <= limit,
        "package source {} grew beyond its size budget",
        path.display()
    );
    *remaining -= contents.len() as u64;
    Ok(String::from_utf8(contents)?)
}

/// Hidden entries below a source folder, such as the `.DS_Store` files Finder leaves behind,
/// are never package sources.
pub(super) fn is_hidden_entry(entry: &walkdir::DirEntry) -> bool {
    entry.depth() > 0 && entry.file_name().to_string_lossy().starts_with('.')
}

#[cfg(test)]
mod package_read_tests {
    use super::*;

    #[test]
    fn rejects_oversized_sources_and_keeps_binary_payloads_out_of_gui_memory() {
        let root =
            std::env::temp_dir().join(format!("chord-package-read-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("js")).unwrap();
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("js/healthy.js"), "export default () => () => {};").unwrap();
        // Sparse files exercise the limits without allocating gigabytes in the test.
        fs::File::create(root.join("bin/helper"))
            .unwrap()
            .set_len(6 * 1024 * 1024 * 1024)
            .unwrap();
        let package = LocalPackageRegistry::import_from_local_folder(&root).unwrap();
        assert!(package.bin_files_contents.is_empty());
        assert_eq!(package.js_files_contents.len(), 1);
        let large = root.join("js/large.js");
        fs::File::create(&large)
            .unwrap()
            .set_len(MAX_SOURCE_BYTES + 1)
            .unwrap();
        let error = LocalPackageRegistry::import_from_local_folder(&root)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("large.js") && error.contains("size budget"),
            "{error}"
        );
        fs::remove_file(large).unwrap();
        let mut remaining = 3;
        assert!(read_package_source(&root.join("js/healthy.js"), &mut remaining).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

#[derive(Serialize, Type)]
pub struct LocalChordPackage {
    path: PathBuf,
}

impl LocalChordPackage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[injectable]
pub struct LocalPackageRegistry {
    handle: AppHandle,
}

impl LocalPackageRegistry {
    pub fn package_store(&self) -> anyhow::Result<super::PackageStore> {
        super::PackageStore::for_app(&self.handle)
    }

    pub fn monorepo_selections(&self) -> anyhow::Result<HashMap<String, Vec<String>>> {
        self.sources_store()?
            .get("monorepoSelections")
            .map(serde_json::from_value)
            .transpose()
            .map(|value| value.unwrap_or_default())
            .context("Failed to read monorepo package selections")
    }

    pub fn monorepo_root(&self, source: &str) -> anyhow::Result<PathBuf> {
        if self.list_monorepos()?.iter().any(|path| path == source) {
            return Ok(PathBuf::from(source));
        }
        let store = self
            .handle
            .app_state()
            .chord_package_manager()
            .registry
            .git
            .git_repos_store
            .store()?;
        let repos = super::git::load_repos(&store)?;
        let repo = repos
            .get(source)
            .filter(|repo| repo.is_monorepo)
            .context("Monorepo is not linked")?;
        Ok(repo
            .linked_local_path
            .as_ref()
            .unwrap_or(&repo.local_abspath)
            .clone())
    }

    pub fn set_monorepo_packages(
        &self,
        source: &str,
        names: Vec<String>,
    ) -> anyhow::Result<Option<Vec<String>>> {
        let root = self.monorepo_root(source)?;
        let available = super::import_monorepo_packages(&self.package_store()?, &root)?;
        super::core::ensure_core_selected(&available, &names)?;
        anyhow::ensure!(
            names.iter().all(|name| available.contains_key(name)),
            "Unknown monorepo package"
        );
        self.save_monorepo_packages(source, Some(names))
    }

    /// Restore the exact saved selection even if a package disappeared during a failed reload.
    pub(in crate::app::chord_package_manager) fn save_monorepo_packages(
        &self,
        source: &str,
        names: Option<Vec<String>>,
    ) -> anyhow::Result<Option<Vec<String>>> {
        let mut selections = self.monorepo_selections()?;
        let previous = match names {
            Some(names) => selections.insert(source.to_owned(), names),
            None => selections.remove(source),
        };
        let store = self.sources_store()?;
        let previous_value = store.get("monorepoSelections");
        store.set("monorepoSelections", serde_json::to_value(selections)?);
        if let Err(error) = store.save() {
            // A failed save must not leave the picker reading an uncommitted selection.
            match previous_value {
                Some(value) => store.set("monorepoSelections", value),
                None => {
                    store.delete("monorepoSelections");
                }
            }
            return Err(error.into());
        }
        Ok(previous)
    }

    pub fn list_monorepos(&self) -> anyhow::Result<Vec<String>> {
        let value = self.sources_store()?.get(LOCAL_MONOREPOS_KEY);
        let mut paths: Vec<String> = value
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        paths.sort();
        Ok(paths)
    }

    pub fn add_monorepo(&self, path: &str) -> anyhow::Result<()> {
        let root = super::validate_monorepo_path(&self.package_store()?, path)?
            .display()
            .to_string();
        let mut paths = self.list_monorepos()?;
        if !paths.contains(&root) {
            paths.push(root);
            paths.sort();
            self.write_monorepos(&paths)?;
        }
        Ok(())
    }

    pub fn remove_monorepo(&self, path: &str) -> anyhow::Result<()> {
        // Use the saved path so a missing/moved checkout can still be unlinked.
        let mut paths = self.list_monorepos()?;
        paths.retain(|saved| saved != path);
        self.write_monorepos(&paths)
    }

    fn write_monorepos(&self, paths: &[String]) -> anyhow::Result<()> {
        let store = self.sources_store()?;
        store.set(LOCAL_MONOREPOS_KEY, serde_json::to_value(paths)?);
        store.save()?;
        Ok(())
    }

    pub fn import_monorepos(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();
        let selections = self.monorepo_selections()?;
        for path in self.list_monorepos()? {
            packages.extend(super::import_selected_monorepo_packages(
                &self.package_store()?,
                Path::new(&path),
                selections.get(&path).map(Vec::as_slice).unwrap_or_default(),
            )?);
        }
        Ok(packages)
    }

    pub fn list_package_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut packages = self
            .read_paths()?
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        packages.sort_by(|left, right| left.cmp(right));
        Ok(packages)
    }

    pub fn pick(&self) -> anyhow::Result<Option<LocalChordPackage>> {
        let Some(folder_path) = self
            .handle
            .dialog()
            .file()
            .set_title("Select Local Chord Folder")
            .blocking_pick_folder()
            .and_then(|folder_path| folder_path.into_path().ok())
        else {
            return Ok(None);
        };

        Ok(Some(self.package_from_user_input(
            &folder_path.display().to_string(),
        )?))
    }

    pub fn add(&self, folder_path: &str) -> anyhow::Result<LocalChordPackage> {
        let package = self.package_from_user_input(folder_path)?;
        let canonical_path = package.path().display().to_string();
        let mut paths = self.read_paths()?;

        if !paths.contains(&canonical_path) {
            paths.push(canonical_path);
            paths.sort();
            self.write_paths(paths.as_slice())?;
        }

        Ok(package)
    }

    pub fn import_all_packages(&self) -> anyhow::Result<HashMap<String, RawChordPackage>> {
        let mut packages = HashMap::new();

        for local_package_path in self.list_package_paths()? {
            if let Ok(package) = self
                .package_store()?
                .import_package(local_package_path.as_path())
                .inspect_err(|e| {
                    log::warn!(
                        "Error importing local folder {}: {e}, skipping",
                        local_package_path.display()
                    );
                })
            {
                packages.insert(package.package_name(), package);
            }
        }

        Ok(packages)
    }

    fn package_from_user_input(&self, path: &str) -> anyhow::Result<LocalChordPackage> {
        let source = self.canonicalize_path(path)?;
        self.package_store()?.mount(&source)?;
        Ok(LocalChordPackage::new(source))
    }

    fn sources_store(&self) -> anyhow::Result<Arc<Store<Wry>>> {
        self.handle
            .store(CHORD_SOURCES_STORE_PATH)
            .context("failed to open chord sources store")
    }

    fn read_paths(&self) -> anyhow::Result<Vec<String>> {
        let store = self.sources_store()?;
        let Some(value) = store.get(LOCAL_FOLDERS_KEY) else {
            return Ok(Vec::new());
        };

        serde_json::from_value(value).context("failed to parse local chord folder list")
    }

    fn write_paths(&self, paths: &[String]) -> anyhow::Result<()> {
        let store = self.sources_store()?;
        let value =
            serde_json::to_value(paths).context("failed to serialize local chord folder list")?;
        store.set(LOCAL_FOLDERS_KEY, value);
        Ok(())
    }

    fn canonicalize_path(&self, path: &str) -> anyhow::Result<PathBuf> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Folder path cannot be empty");
        }

        let canonical_path =
            fs::canonicalize(trimmed).context(format!("failed to access folder {trimmed}"))?;
        if !canonical_path.is_dir() {
            anyhow::bail!("{trimmed} is not a folder");
        }

        Ok(canonical_path)
    }

    #[cfg(test)]
    pub fn import_from_local_folder(root: &Path) -> anyhow::Result<RawChordPackage> {
        read_package(root)
    }
}

/// Low-level reader. Production callers must go through PackageStore, which mounts sources
/// and checks their logical root before any manifest, chords, or JavaScript is read.
pub(super) fn read_package(root: &Path) -> anyhow::Result<RawChordPackage> {
    let mut chords_files_contents = HashMap::new();
    let mut js_files_contents = HashMap::new();
    let bin_files_contents = HashMap::new();
    let mut remaining = MAX_PACKAGE_SOURCE_BYTES;

    let dirname = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let manifest = root.join("package.json");
    let package_json_contents = if manifest.exists() {
        Some(read_package_source(&manifest, &mut remaining)?)
    } else {
        None
    };

    // Native modules under `target/` are not read: the package's own JS loads them from
    // disk with `process.dlopen` (see `import.meta.chord.resolveNative`).
    // Executables in bin/ are also consumed from disk by workers; their bytes never
    // need to be copied into the GUI's memory.
    let mut entries = 0;
    for dir in ["chords", "js"] {
        let dir_path = root.join(dir);
        if !dir_path.exists() {
            continue;
        }

        for entry in WalkDir::new(&dir_path)
            .into_iter()
            .filter_entry(|entry| !is_hidden_entry(entry))
        {
            let entry = entry?;
            entries += 1;
            anyhow::ensure!(
                entries <= 10_000,
                "package {} contains too many source entries",
                root.display()
            );
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let relpath = path.strip_prefix(root)?;

            match dir {
                "chords" => {
                    let content = read_package_source(path, &mut remaining)?;
                    chords_files_contents.insert(relpath.to_path_buf(), content);
                }
                "js" => {
                    if path.extension().is_some_and(|ext| ext == "js") {
                        let content = read_package_source(path, &mut remaining)?;
                        js_files_contents.insert(relpath.to_path_buf(), content);
                    }
                }
                _ => {}
            }
        }
    }

    log::debug!(
        "loaded chord package from {:?}:\njs: {:?}\nchords: {:?}\nbin: {:?}",
        root,
        js_files_contents.keys(),
        chords_files_contents.keys(),
        bin_files_contents.keys()
    );

    Ok(RawChordPackage {
        dirname,
        root: root.to_path_buf(),
        package_json_contents,
        chords_files_contents,
        js_files_contents,
        bin_files_contents,
    })
}

#[allow(dead_code)]
fn is_supported_macos_chord_filename(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    file_name == "macos.toml" || file_name.ends_with(".macos.toml")
}
