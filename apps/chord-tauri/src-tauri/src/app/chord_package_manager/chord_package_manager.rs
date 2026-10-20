use crate::app::AppHandleExt;
use crate::app::chord_package_manager::chord_package_registry::{
    ChordPackageRegistry, ImportedPackages,
};
use crate::app::chord_package_manager::{
    ChordJsPackage, ChordPackage, ChordReference, chords_file_app_id,
};
use crate::app::state::AppSingleton;
use crate::models::{
    ChordEventTrigger, ChordInput, ChordInputEvent, ChordsFileImportOverride, CompiledChordsFile,
    CompiledChordsFileHandler, FilePathslug, ParsedChordsFile, RawChordPackage, RawChordsFile,
    CORE_CHORD_PACKAGE, is_core_chords_file,
};
use crate::state::{
    ChordPackageManagerObservable, ChordPackageManagerState, GitReposObservable, Observable,
};
use anyhow::{Context, Result};
use nject::{inject, injectable};
use ordermap::OrderMap;
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;

#[injectable]
pub struct ChordPackageManager {
    pub registry: ChordPackageRegistry,

    /// An ordered mapping from package name to package. Uses an OrderMap to allow the user to
    /// prioritize certain packages
    #[inject(RwLock::new(OrderMap::new()))]
    packages: RwLock<OrderMap<String, ChordPackage>>,

    /// On-disk root of every package whose JS has been loaded, by package name. Filled before
    /// the package's handlers are compiled (unlike `packages`, which only lists finished
    /// packages) so the `chord` module can resolve package files while a module is importing.
    #[inject(RwLock::new(HashMap::new()))]
    package_roots: RwLock<HashMap<String, PathBuf>>,

    #[inject(tokio::sync::Mutex::new(()))]
    reload_lock: tokio::sync::Mutex<()>,

    #[inject(AtomicU64::new(0))]
    reload_requested: AtomicU64,

    #[inject(AtomicU64::new(0))]
    reload_completed: AtomicU64,

    #[inject(Mutex::new(None))]
    last_reload_error: Mutex<Option<String>>,

    /// App IDs that chords files claim for bare executables through `executables`, keyed by
    /// executable file name.
    #[inject(RwLock::new(HashMap::new()))]
    executable_app_ids: RwLock<HashMap<String, String>>,

    observable: ChordPackageManagerObservable,
    handle: AppHandle,
}

struct ChordInputEventContext {
    chord_package: ChordPackage,
    event: ChordInputEvent,
}

fn resolve_handler_args(
    args: &[toml::Value],
    chords_file: &ParsedChordsFile,
    import_override: Option<&ChordsFileImportOverride>,
) -> Result<Vec<toml::Value>> {
    args.iter()
        .map(|arg| {
            if let Some(name) = arg.as_str().filter(|name| name.starts_with('$')) {
                return import_override
                    .and_then(|value| value.meta.get(name))
                    .or_else(|| chords_file.meta.get(name))
                    .cloned()
                    .with_context(|| format!("missing arg {name}"));
            }
            Ok(arg.clone())
        })
        .collect()
}

#[cfg(test)]
mod handler_args_tests {
    use super::*;

    #[test]
    fn import_meta_overrides_handler_args() {
        let base: ParsedChordsFile = r#"
name = "JetBrains Base Chords"
[meta]
'$execPath' = "default-executable"
[on.command]
file = "jetbrains.js"
args = ['$execPath', "literal", 42, true]
"#
        .parse()
        .unwrap();
        let rustrover: ParsedChordsFile = r#"
name = "RustRover"
[[import]]
file = "base.toml"
[import.override.meta]
'$execPath' = "/Applications/RustRover.app/Contents/MacOS/rustrover"
"#
        .parse()
        .unwrap();
        let args = &base.handlers["command"].args;
        let resolved =
            resolve_handler_args(args, &base, rustrover.imports[0].r#override.as_ref()).unwrap();
        assert_eq!(
            resolved[0].as_str(),
            Some("/Applications/RustRover.app/Contents/MacOS/rustrover")
        );
        assert_eq!(&resolved[1..], &args[1..]);
        assert_eq!(
            resolve_handler_args(args, &base, None).unwrap()[0].as_str(),
            Some("default-executable")
        );
        let missing = vec![toml::Value::String("$missing".into())];
        assert_eq!(
            resolve_handler_args(&missing, &base, None)
                .unwrap_err()
                .to_string(),
            "missing arg $missing"
        );
    }
}

/// Claims follow package priority; a later package cannot take over an executable.
fn executable_app_ids<'a>(
    packages: impl IntoIterator<Item = &'a ChordPackage>,
) -> HashMap<String, String> {
    let mut app_ids = HashMap::<String, String>::new();
    for package in packages {
        let mut files: Vec<_> = package.compiled_chords_files.values().collect();
        files.sort_by(|a, b| a.pathslug.cmp(&b.pathslug));
        for file in files {
            if file.executables.is_empty() {
                continue;
            }
            let Some(app_id) = chords_file_app_id(&file.pathslug) else {
                log::warn!(
                    "ignoring executables in {} from {}: only app chords files can claim executables",
                    file.pathslug.display(),
                    package.name
                );
                continue;
            };
            for executable in &file.executables {
                match app_ids.get(executable) {
                    Some(existing) if existing != &app_id => log::warn!(
                        "executable {executable:?} is claimed by {existing} and {app_id} ({}); using {existing}",
                        package.name
                    ),
                    Some(_) => {}
                    None => {
                        app_ids.insert(executable.clone(), app_id.clone());
                    }
                }
            }
        }
    }
    app_ids
}

fn handler_ids(packages: &OrderMap<String, ChordPackage>) -> HashSet<String> {
    packages
        .values()
        .flat_map(|package| package.compiled_chords_files.values())
        .flat_map(|file| file.handlers.iter())
        .map(|handler| handler.handler_id.clone())
        .collect()
}

fn parse_package_chords_file(path: &Path, contents: &str) -> Result<ParsedChordsFile> {
    contents
        .parse()
        .with_context(|| format!("Failed to parse {}", path.display()))
}

fn imported_chords_path(pathslug: &Path, file: &Path) -> PathBuf {
    let bundled = pathslug
        .components()
        .nth(1)
        .and_then(|part| part.as_os_str().to_str())
        .is_some_and(|part| part.starts_with('@'));
    if bundled {
        pathslug
            .components()
            .take(3)
            .collect::<PathBuf>()
            .join("chords")
            .join(file)
    } else {
        Path::new("chords").join(file)
    }
}

/// Validate without Rust recursion: a package import cycle otherwise overflows the GUI stack.
fn validate_chords_imports(root: &Path, files: &HashMap<PathBuf, ParsedChordsFile>) -> Result<()> {
    let mut pending = vec![(root.to_path_buf(), false)];
    let mut visiting = HashSet::new();
    let mut visits = 0;
    let mut items = 0;
    while let Some((path, exit)) = pending.pop() {
        if exit {
            visiting.remove(&path);
            continue;
        }
        anyhow::ensure!(
            visiting.insert(path.clone()),
            "cyclic chords import at {}",
            path.display()
        );
        anyhow::ensure!(
            visiting.len() <= 64,
            "chords imports exceed 64 levels at {}",
            path.display()
        );
        visits += 1;
        let file = files
            .get(&path)
            .with_context(|| format!("import file {} not found", path.display()))?;
        items += file.chords.len() + file.handlers.len() + file.chord_hints.len();
        anyhow::ensure!(
            visits + pending.len() + file.imports.len() <= 4096 && items <= 100_000,
            "chords import expansion is too large at {}",
            root.display()
        );
        pending.push((path.clone(), true));
        for import in file.imports.iter().rev() {
            pending.push((imported_chords_path(&path, Path::new(&import.file)), false));
        }
    }
    Ok(())
}

fn ensure_selected_packages_loaded(
    names: &[String],
    package_errors: &HashMap<String, Vec<String>>,
) -> Result<()> {
    let mut errors = Vec::new();
    for name in names {
        match package_errors.get(name) {
            Some(details) if details.is_empty() => {}
            Some(details) => {
                let mut details = details.clone();
                details.sort();
                errors.push(format!("Cannot enable {name}:\n{}", details.join("\n\n")));
            }
            None => errors.push(format!("Cannot enable {name}: package was not loaded")),
        }
    }
    anyhow::ensure!(errors.is_empty(), "{}", errors.join("\n\n"));
    Ok(())
}

#[cfg(test)]
mod package_selection_tests {
    use super::*;

    #[test]
    fn duplicate_chord_keys_report_the_file_and_reject_selection() {
        let path = Path::new("chords/com/apple/systempreferences/macos.toml");
        let contents = r#"
name = "System Preferences"
[chords]
'/,' = { name = "System Preferences", shell = "open -b com.apple.systempreferences" }
'gwa' = { name = "Goto: Wallpaper" }
'gwa' = { name = "Goto: Wallet & Apple Pay" }
"#;
        let error = parse_package_chords_file(path, contents).unwrap_err();
        let details = format!("{error:#}");
        assert!(details.contains(&path.display().to_string()));
        assert!(details.contains("duplicate key"));
        assert!(details.contains("'gwa'"));
        let name = "@keychord/chords-com.apple".to_owned();
        let error = ensure_selected_packages_loaded(
            std::slice::from_ref(&name),
            &HashMap::from([(name.clone(), vec![details])]),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(&format!("Cannot enable {name}")));
        assert!(error.contains("duplicate key"));
    }

    #[test]
    fn invalid_chord_definitions_are_reported_after_toml_parsing() {
        let path = Path::new("chords/macos.toml");
        let contents = r#"
name = "Invalid handler"
[on.input]
file = 42
"#;
        // The raw representation accepts this TOML, but normalization rejects the handler.
        toml::from_str::<RawChordsFile>(contents).unwrap();
        let error = format!(
            "{:#}",
            parse_package_chords_file(path, contents).unwrap_err()
        );
        assert!(error.contains("chords/macos.toml"));
        assert!(error.contains("handler must have the file key"));
    }

    #[test]
    fn compilation_failures_reject_only_newly_selected_packages() {
        let healthy = "@test/healthy".to_owned();
        let broken = "@test/broken".to_owned();
        let package_errors = HashMap::from([
            (healthy.clone(), Vec::new()),
            (
                broken.clone(),
                vec!["Failed to compile chords/macos.toml: JS default export did not resolve to a function: undefined".to_owned()],
            ),
        ]);
        ensure_selected_packages_loaded(&[healthy], &package_errors).unwrap();
        // Removing a broken package and ordinary reloads must remain possible.
        ensure_selected_packages_loaded(&[], &package_errors).unwrap();
        let error = ensure_selected_packages_loaded(&[broken], &package_errors)
            .unwrap_err()
            .to_string();
        assert!(error.contains("@test/broken"));
        assert!(error.contains("chords/macos.toml"));
        assert!(error.contains("JS default export"));
    }

    #[test]
    fn missing_packages_cannot_be_accepted_as_loaded() {
        let error = ensure_selected_packages_loaded(&["@test/missing".to_owned()], &HashMap::new())
            .unwrap_err()
            .to_string();
        assert_eq!(error, "Cannot enable @test/missing: package was not loaded");
    }

    #[test]
    fn valid_chords_can_be_loaded() {
        let parsed = parse_package_chords_file(
            Path::new("chords/macos.toml"),
            "name = 'Valid'\n[chords]\n'/,' = { shell = 'open -b com.apple.systempreferences' }",
        )
        .unwrap();
        assert_eq!(parsed.chords.len(), 1);
        assert_eq!(parsed.chords[0].raw_trigger, "/,");
    }

    #[test]
    fn cycles_and_exponential_imports_are_rejected_before_compilation() {
        let parse = |source: &str| source.parse::<ParsedChordsFile>().unwrap();
        let a = PathBuf::from("chords/a.toml");
        let b = PathBuf::from("chords/b.toml");
        let cyclic = HashMap::from([
            (a.clone(), parse("name = 'a'\n[[import]]\nfile = 'b.toml'")),
            (b.clone(), parse("name = 'b'\n[[import]]\nfile = 'a.toml'")),
        ]);
        assert!(
            validate_chords_imports(&a, &cyclic)
                .unwrap_err()
                .to_string()
                .contains("cyclic chords import")
        );
        let mut expanding = HashMap::new();
        for index in 0..15 {
            let source = if index == 14 {
                "name = 'leaf'".into()
            } else {
                format!(
                    "name = 'branch'\n[[import]]\nfile = '{}.toml'\n[[import]]\nfile = '{}.toml'",
                    index + 1,
                    index + 1
                )
            };
            expanding.insert(
                PathBuf::from(format!("chords/{index}.toml")),
                parse(&source),
            );
        }
        assert!(
            validate_chords_imports(Path::new("chords/0.toml"), &expanding)
                .unwrap_err()
                .to_string()
                .contains("expansion is too large")
        );
    }

    #[test]
    fn valid_diamond_and_vendored_imports_keep_their_paths() {
        let parse = |source: &str| source.parse::<ParsedChordsFile>().unwrap();
        let files = HashMap::from([
            (
                PathBuf::from("chords/root.toml"),
                parse("name = 'root'\n[[import]]\nfile = 'a.toml'\n[[import]]\nfile = 'b.toml'"),
            ),
            (
                PathBuf::from("chords/a.toml"),
                parse("name = 'a'\n[[import]]\nfile = 'leaf.toml'"),
            ),
            (
                PathBuf::from("chords/b.toml"),
                parse("name = 'b'\n[[import]]\nfile = 'leaf.toml'"),
            ),
            (PathBuf::from("chords/leaf.toml"), parse("name = 'leaf'")),
        ]);
        validate_chords_imports(Path::new("chords/root.toml"), &files).unwrap();
        assert_eq!(
            imported_chords_path(
                Path::new("chords/@scope/package/chords/root.toml"),
                Path::new("leaf.toml")
            ),
            PathBuf::from("chords/@scope/package/chords/leaf.toml")
        );
    }
}

impl ChordPackageManager {
    pub(super) fn init_core_package(&self) -> Result<()> {
        let package = super::chord_package_registry::core::load_core_package()?;
        let (hints, halt_template) = super::chord_package_registry::core::meta_configuration(&package)?;
        self.packages.write().insert(package.name.clone(), package.clone());
        self.update_executable_app_ids();
        self.observable.set_state(|_| ChordPackageManagerState {
            packages: vec![package],
            ..Default::default()
        })?;
        self.handle.app_state().chord_mode_manager().configure_meta_commands(hints, halt_template)
    }

    pub async fn set_monorepo_packages(&self, source: &str, names: Vec<String>) -> Result<()> {
        // Keep selection changes and their rollback serialized with every other reload.
        let _reload_guard = self.reload_lock.lock().await;
        let previous = self
            .registry
            .local
            .set_monorepo_packages(source, names.clone())?;
        let newly_selected = names
            .into_iter()
            .filter(|name| !previous.as_ref().is_some_and(|names| names.contains(name)))
            .collect::<Vec<_>>();

        if let Err(error) = self.reload_once(&newly_selected).await {
            self.registry
                .local
                .save_monorepo_packages(source, previous)
                .map_err(|restore_error| {
                    anyhow::anyhow!(
                        "{error:#}\n\nFailed to restore package selection: {restore_error:#}"
                    )
                })?;
            // Compilation can register JS handlers and lifecycle callbacks. Rebuild the old
            // selection to clean up those registrations as well as package roots.
            self.reload_once(&[]).await.map_err(|restore_error| {
                anyhow::anyhow!(
                    "{error:#}\n\nFailed to reload previous package selection: {restore_error:#}"
                )
            })?;
            return Err(error);
        }
        Ok(())
    }

    pub async fn reload_all(&self) -> Result<()> {
        let request = self.reload_requested.fetch_add(1, Ordering::SeqCst) + 1;
        let _reload_guard = self.reload_lock.lock().await;

        if self.reload_completed.load(Ordering::SeqCst) >= request {
            return self.last_reload_result();
        }

        // Include every request that arrived while this caller was waiting. Requests arriving
        // during the reload will be handled by one trailing caller instead of creating a queue of
        // complete reloads.
        let generation = self.reload_requested.load(Ordering::SeqCst);
        let result = self.reload_once(&[]).await;
        *self.last_reload_error.lock() = result.as_ref().err().map(|error| format!("{error:#}"));
        self.reload_completed.store(generation, Ordering::SeqCst);
        result
    }

    fn last_reload_result(&self) -> Result<()> {
        match self.last_reload_error.lock().clone() {
            Some(error) => Err(anyhow::anyhow!(error)),
            None => Ok(()),
        }
    }

    async fn reload_once(&self, required_packages: &[String]) -> Result<()> {
        self.handle.app_state().chord_mode_manager().stop_loops()?;
        let ImportedPackages {
            packages: raw_chord_packages,
            personal: personal_packages,
        } = self.registry.import_all_packages()?;
        let previous_roots = self.package_roots.read().clone();

        let next_roots = raw_chord_packages
            .values()
            .filter(|package| !package.js_files_contents.is_empty())
            .map(|package| (package.package_name(), package.root.clone()))
            .collect::<HashMap<_, _>>();
        let mut loading_roots = previous_roots.clone();
        loading_roots.extend(next_roots.clone());
        *self.package_roots.write() = loading_roots;

        // Packages earlier in the order win within each scope, so personal packages come first,
        // in their own priority order.
        let personal_ranks = personal_packages
            .iter()
            .filter_map(|state| state.name.clone())
            .enumerate()
            .map(|(rank, name)| (name, rank))
            .collect::<HashMap<_, _>>();
        let mut raw_chord_packages = raw_chord_packages.into_iter().collect::<Vec<_>>();
        raw_chord_packages
            .sort_by_key(|(name, _)| personal_ranks.get(name).copied().unwrap_or(usize::MAX));

        let mut chord_packages = OrderMap::new();
        let mut package_errors = HashMap::new();
        for (package_name, raw_chord_package) in raw_chord_packages {
            match self.load_package(raw_chord_package).await {
                Ok((package, errors)) => {
                    package_errors.insert(package_name, errors);
                    chord_packages.insert(package.name.clone(), package);
                }
                Err(error) => {
                    log::error!(
                        "skipping package {} because of loading error: {:?}",
                        package_name,
                        error
                    );
                    package_errors.insert(package_name, vec![format!("{error:#}")]);
                }
            }
        }

        // Ordinary reloads retain usable files from partially broken packages. A newly
        // selected package must load completely before its selection can be accepted.
        ensure_selected_packages_loaded(required_packages, &package_errors)?;
        let core = chord_packages.get(CORE_CHORD_PACKAGE).context("Chord's core package must remain loaded")?;
        let (hints, halt_template) = super::chord_package_registry::core::meta_configuration(core)?;

        let next_handler_ids = handler_ids(&chord_packages);
        let observable_packages = chord_packages.values().cloned().collect();

        *self.packages.write() = chord_packages;
        *self.package_roots.write() = next_roots;
        self.update_executable_app_ids();
        self.handle.app_state().chord_mode_manager().configure_meta_commands(hints, halt_template)?;
        package_errors.retain(|_, errors| !errors.is_empty());
        let observable_result = self.observable.set_state(|_| ChordPackageManagerState {
            packages: observable_packages,
            errors: package_errors,
            personal_packages,
        });

        if let Err(error) =
            bun_handlers::retain_runtime_registrations(self.handle.clone(), next_handler_ids).await
        {
            log::warn!("Failed to prune stale Bun registrations after reload: {error:#}");
        }

        observable_result?;
        Ok(())
    }

    pub fn get_package_by_name(&self, package_name: &str) -> Option<ChordPackage> {
        self.packages.read().get(package_name).cloned()
    }

    /// The app ID a loaded chords file claims for a bare executable with this file name.
    pub fn app_id_for_executable(&self, executable_name: &str) -> Option<String> {
        self.executable_app_ids.read().get(executable_name).cloned()
    }

    /// Rebuilds the executable claims, then re-resolves the frontmost app, which may be a bare
    /// executable that a changed package now claims.
    fn update_executable_app_ids(&self) {
        let app_ids = executable_app_ids(self.packages.read().values());
        *self.executable_app_ids.write() = app_ids;
        if let Err(error) = self.handle.app_state().frontmost().refresh() {
            log::error!("failed to re-resolve the frontmost app: {error:#}");
        }
    }

    /// On-disk root of a package whose JS has been loaded (see `package_roots`).
    pub fn package_root(&self, package_name: &str) -> Option<PathBuf> {
        self.package_roots.read().get(package_name).cloned()
    }

    /// The loaded package containing `path` (the longest matching root wins), as
    /// `(package name, root)`.
    pub fn package_for_path(&self, path: &Path) -> Option<(String, PathBuf)> {
        self.package_roots
            .read()
            .iter()
            .filter(|(_, root)| path.starts_with(root))
            .max_by_key(|(_, root)| root.as_os_str().len())
            .map(|(name, root)| (name.clone(), root.clone()))
    }

    async fn load_package(
        &self,
        raw_chord_package: RawChordPackage,
    ) -> Result<(ChordPackage, Vec<String>)> {
        let name = raw_chord_package.package_name();
        log::debug!("loading package {}", name);
        let root = raw_chord_package.root.clone();

        let mut raw_chords_files = HashMap::new();
        let mut compiled_chords_files = HashMap::new();
        let mut global_chords = Vec::new();
        let mut parsed_chords_files = HashMap::new();
        let mut errors = Vec::new();

        for (path, contents) in raw_chord_package.chords_files_contents {
            let Ok(raw) = toml::from_str::<RawChordsFile>(&contents)
                .with_context(|| format!("Failed to parse {}", path.display()))
                .inspect_err(|error| {
                    log::error!("error when loading package {name}: {error:#}");
                    errors.push(format!("{error:#}"));
                })
            else {
                continue;
            };
            raw_chords_files.insert(path.clone(), raw);
            let parsed = if is_core_chords_file(&name, &path) {
                ParsedChordsFile::parse_core(&contents)
                    .with_context(|| format!("Failed to parse protected core file {}", path.display()))
            } else { parse_package_chords_file(&path, &contents) };
            match parsed {
                Ok(parsed) => {
                    parsed_chords_files.insert(path, parsed);
                }
                Err(error) => {
                    log::error!("error when loading package {name}: {error:#}");
                    errors.push(format!("{error:#}"));
                }
            }
        }

        let js_package = self
            .load_js_package(&name, root, raw_chord_package.js_files_contents)
            .await?;

        // Compile sequentially inside this package's bounded worker queue.
        for (pathslug, parsed_chord_file) in &parsed_chords_files {
            let compile_result = Self::compile_chords_file(
                self.handle.clone(),
                parsed_chord_file,
                pathslug.clone(),
                &js_package,
                &parsed_chords_files,
                &None,
            )
            .await;

            match compile_result {
                Ok(chords_file) => {
                    log::debug!(
                        "compiled chords file {:#?} with {} chords",
                        Path::new(&name).join(pathslug),
                        chords_file.chords.len()
                    );

                    let is_bundled_chords_file = pathslug
                        .components()
                        .nth(1)
                        .and_then(|c| c.as_os_str().to_str())
                        .map(|s| s.starts_with('@'))
                        .unwrap_or(false);
                    if !is_bundled_chords_file {
                        // We only want to add global chords from non-bundled chord files (i.e. pathslugs that
                        // don't start with `chords/@`
                        for chord in &chords_file.chords {
                            let first_char = chord.raw_trigger.chars().next();
                            let is_non_alphanumeric =
                                first_char.map(|c| !c.is_alphanumeric()).unwrap_or(false);

                            if is_non_alphanumeric || is_core_chords_file(&name, pathslug) {
                                global_chords.push(ChordReference {
                                    package_name: name.clone(),
                                    chords_file_pathslug: pathslug.clone(),
                                    chord: chord.clone(),
                                });
                            }
                        }
                    }

                    compiled_chords_files.insert(pathslug.clone(), chords_file);
                }
                Err(e) => {
                    log::error!(
                        "skipping chords file {:?} in {} because of compilation error: {:#}",
                        pathslug,
                        name,
                        e
                    );
                    errors.push(format!("Failed to compile {}: {e:#}", pathslug.display()));
                }
            }
        }

        let chord_package = ChordPackage {
            name: name.clone(),
            js_package,
            compiled_chords_files,
            raw_chords_files,
            global_chords,
        };

        Ok((chord_package, errors))
    }

    async fn load_js_package(
        &self,
        package_name: &str,
        root: PathBuf,
        files: HashMap<FilePathslug, String>,
    ) -> Result<Option<ChordJsPackage>> {
        log::debug!("loading JS package {}", package_name);

        if files.is_empty() {
            log::debug!("JS package {} was empty", package_name);
            return Ok(None);
        }

        let package = ChordJsPackage::builder(self.handle.clone(), package_name, root)
            .load(files)
            .await?;
        Ok(Some(package))
    }

    pub async fn compile_chords_file(
        handle: AppHandle,
        chords_file: &ParsedChordsFile,
        pathslug: FilePathslug,
        js_package: &Option<ChordJsPackage>,
        parsed_chords_files: &HashMap<PathBuf, ParsedChordsFile>,
        r#override: &Option<ChordsFileImportOverride>,
    ) -> Result<CompiledChordsFile> {
        validate_chords_imports(&pathslug, parsed_chords_files)?;
        Self::compile_validated_chords_file(
            handle,
            chords_file,
            pathslug,
            js_package,
            parsed_chords_files,
            r#override,
        )
        .await
    }

    async fn compile_validated_chords_file(
        handle: AppHandle,
        chords_file: &ParsedChordsFile,
        pathslug: FilePathslug,
        js_package: &Option<ChordJsPackage>,
        parsed_chords_files: &HashMap<PathBuf, ParsedChordsFile>,
        r#override: &Option<ChordsFileImportOverride>,
    ) -> Result<CompiledChordsFile> {
        log::debug!("compiling chords file {}", chords_file.name);

        let mut chords = chords_file.chords.clone();
        let mut chord_hints = chords_file.chord_hints.clone();
        let mut handlers = Vec::new();
        for (event, handler) in &chords_file.handlers {
            let build_args = resolve_handler_args(&handler.args, chords_file, r#override.as_ref())?;

            let file = handler.file.clone();
            let raw = chords_file.raw.clone();
            let pathslug_string = pathslug
                .to_str()
                .context("failed to get pathslug as str")?
                .to_string();
            let bundle_id = pathslug
                .parent()
                .and_then(|p| p.strip_prefix("chords").ok())
                .map(|p| p.to_str())
                .context("failed to get pathslug as str")?
                .map(|p| p.to_string().replace("/", "."));
            let Some(js_package) = js_package else {
                anyhow::bail!(
                    "handler {event}: a JS package (js/ directory) must be present when defining a JS handler"
                )
            };

            // Bun imports the module from disk, so `import.meta` and native module resolution
            // see the package directory.
            let Some(module_path) = js_package.resolve_file_path(&pathslug, &file)? else {
                anyhow::bail!("file {} not found in js package {}", file, js_package.name);
            };
            let handler_id = bun_handlers::register_handler(
                handle.clone(),
                module_path.to_string_lossy().into_owned(),
                raw,
                pathslug_string,
                bundle_id,
                build_args,
            )
            .await?;

            handlers.push(CompiledChordsFileHandler {
                event: event.clone(),
                handler_id,
            });
        }

        for import in &chords_file.imports {
            let imported_file_path = imported_chords_path(&pathslug, Path::new(&import.file));

            let imported_file = parsed_chords_files
                .get(&imported_file_path)
                .context(format!("import file {:?} not found", imported_file_path))?;
            log::debug!(
                "resolved import file {:?} from path {:?}",
                imported_file.name,
                imported_file_path
            );

            let compiled_file = Box::pin(Self::compile_validated_chords_file(
                handle.clone(),
                imported_file,
                imported_file_path,
                js_package,
                parsed_chords_files,
                &import.r#override,
            ))
            .await?;
            chords.extend(compiled_file.chords.clone());
            chord_hints.extend(compiled_file.chord_hints.clone());
            handlers.extend(compiled_file.handlers.clone());
        }

        log::debug!("finished compiling chords file {}", chords_file.name);

        Ok(CompiledChordsFile {
            name: chords_file.name.clone(),
            pathslug,
            meta: chords_file.meta.clone(),
            handlers,
            executables: chords_file.executables.clone(),
            chords,
            chord_hints,
        })
    }

    /// Gets the chord package that is responsible for handling a specific chord input event
    pub fn create_event_context(&self, event: &ChordInputEvent) -> Option<ChordPackage> {
        self.matching_chord_contexts(event)
            .into_iter()
            .find(|(_, reference)| {
                reference
                    .chord
                    .actions
                    .iter()
                    .any(|action| action.trigger() == ChordEventTrigger::Input)
            })
            .map(|(package, _)| package)
    }

    /// Lifecycle events visit all matching chords, in app/global and package priority order.
    pub fn matching_chord_contexts(
        &self,
        event: &ChordInputEvent,
    ) -> Vec<(ChordPackage, ChordReference)> {
        let packages = self.packages.read();
        collect_matching_chord_contexts(
            packages.values(),
            event,
            self.handle.app_state().frontmost().file_picker_active(),
        )
    }
}

fn collect_matching_chord_contexts<'a>(
    packages: impl IntoIterator<Item = &'a ChordPackage>,
    event: &ChordInputEvent,
    file_picker_active: bool,
) -> Vec<(ChordPackage, ChordReference)> {
    let packages: Vec<_> = packages.into_iter().collect();
    let mut scopes = Vec::new();
    if file_picker_active {
        scopes.push(Some("com.apple.finder"));
    }
    if let Some(app_id) = event.application_id.as_deref() {
        scopes.push(Some(app_id));
    }
    scopes.push(None);
    let mut contexts = Vec::new();
    let mut seen = HashSet::new();
    for scope in scopes {
        for package in &packages {
            let matches = match scope {
                Some(app_id) => package.matching_app_chords_for_id(event, app_id),
                None => package.matching_global_chords(event),
            };
            for reference in matches {
                let identity = (
                    reference.package_name.clone(),
                    reference.chords_file_pathslug.clone(),
                    reference.chord.index,
                    reference.chord.raw_trigger.clone(),
                );
                if seen.insert(identity) {
                    contexts.push(((*package).clone(), reference));
                }
            }
        }
    }
    contexts
}

#[cfg(test)]
mod executable_app_ids_tests {
    use super::*;

    fn package(name: &str, files: &[(&str, &str)]) -> ChordPackage {
        let compiled_chords_files = files
            .iter()
            .map(|(path, contents)| {
                let parsed: ParsedChordsFile = contents.parse().unwrap();
                let path = PathBuf::from(path);
                let file = CompiledChordsFile {
                    name: parsed.name,
                    pathslug: path.clone(),
                    meta: parsed.meta,
                    handlers: vec![],
                    executables: parsed.executables,
                    chords: parsed.chords,
                    chord_hints: parsed.chord_hints,
                };
                (path, file)
            })
            .collect();
        ChordPackage {
            name: name.into(),
            js_package: None,
            raw_chords_files: HashMap::new(),
            compiled_chords_files,
            global_chords: vec![],
        }
    }

    #[test]
    fn app_chords_files_claim_executables_in_package_priority_order() {
        let livesplit = package(
            "livesplit",
            &[(
                "chords/org/livesplit/one/macos.toml",
                "name = 'LiveSplit One'\nexecutables = ['LiveSplit One', 'LiveSplit One Nightly']",
            )],
        );
        let later = package(
            "later",
            &[
                (
                    "chords/dev/other/macos.toml",
                    "name = 'Other'\nexecutables = ['LiveSplit One']",
                ),
                // Root and bundled files apply to no app, so they cannot claim executables.
                (
                    "chords/macos.toml",
                    "name = 'Global'\nexecutables = ['Global Tool']",
                ),
                (
                    "chords/@scope/pkg/chords/dev/tool/macos.toml",
                    "name = 'Bundled'\nexecutables = ['Bundled Tool']",
                ),
            ],
        );
        let app_ids = executable_app_ids([&livesplit, &later]);
        assert_eq!(
            app_ids,
            HashMap::from([
                ("LiveSplit One".into(), "org.livesplit.one".into()),
                ("LiveSplit One Nightly".into(), "org.livesplit.one".into()),
            ])
        );
        assert_eq!(
            executable_app_ids([&later, &livesplit])
                .get("LiveSplit One")
                .map(String::as_str),
            Some("dev.other")
        );
    }

    #[test]
    fn chords_file_app_ids_mirror_app_chord_matching() {
        assert_eq!(
            chords_file_app_id(Path::new("chords/com/spotify/client/macos.toml")).as_deref(),
            Some("com.spotify.client")
        );
        assert_eq!(
            chords_file_app_id(Path::new("chords/livesplit_one/macos.toml")).as_deref(),
            Some("livesplit_one")
        );
        for path in [
            "chords/macos.toml",
            "chords/@scope/pkg/chords/com/x/macos.toml",
            "chords/com/x/shared.toml",
        ] {
            assert_eq!(chords_file_app_id(Path::new(path)), None, "{path}");
        }
    }
}

#[cfg(test)]
mod file_picker_tests {
    use super::*;
    use crate::models::Key;

    fn package(name: &str, app_id: &str, bindings: &str) -> ChordPackage {
        let parsed: ParsedChordsFile = format!("name = '{name}'\n[chords]\n{bindings}")
            .parse()
            .unwrap();
        let path = PathBuf::from(format!("chords/{}/macos.toml", app_id.replace('.', "/")));
        let file = CompiledChordsFile {
            name: parsed.name,
            pathslug: path.clone(),
            meta: parsed.meta,
            chords: parsed.chords,
            chord_hints: parsed.chord_hints,
            handlers: vec![],
            executables: parsed.executables,
        };
        ChordPackage {
            name: name.into(),
            js_package: None,
            raw_chords_files: HashMap::new(),
            compiled_chords_files: HashMap::from([(path, file)]),
            global_chords: vec![],
        }
    }

    fn event(input: &str, app: &str) -> ChordInputEvent {
        ChordInputEvent {
            input: Key::parse_sequence(input).unwrap(),
            application_id: Some(app.into()),
            ..Default::default()
        }
    }

    #[test]
    fn picker_precedes_host_app_and_global_even_when_finder_package_is_last() {
        let mut host = package(
            "host",
            "dev.fixture",
            "gh = { name = 'Host', shortcut = 'cmd+h' }",
        );
        let finder = package(
            "finder",
            "com.apple.finder",
            "gh = { name = 'Home', shortcut = 'cmd+shift+h' }",
        );
        let mut global = host
            .matching_app_chords(&event("gh", "dev.fixture"))
            .remove(0);
        global.chord.name = "Global".into();
        global.chords_file_pathslug = PathBuf::from("chords/macos.toml");
        host.global_chords.push(global);
        let input = event("gh", "dev.fixture");
        let matches = collect_matching_chord_contexts([&host, &finder], &input, true);
        let names: Vec<_> = matches
            .iter()
            .map(|(_, reference)| reference.chord.name.as_str())
            .collect();
        assert_eq!(names, ["Home", "Host", "Global"]);
        assert_eq!(input.application_id.as_deref(), Some("dev.fixture"));
        // The picked shortcut still executes in the real foreground context.
        let task = matches[0]
            .0
            .resolve_task(input, matches[0].1.clone(), 1)
            .unwrap()
            .unwrap();
        assert_eq!(task.event.application_id.as_deref(), Some("dev.fixture"));
    }

    #[test]
    fn leaving_picker_restores_host_priority_and_unmatched_picker_chords_fall_back() {
        let host = package(
            "host",
            "dev.fixture",
            "gh = { name = 'Host', shortcut = 'cmd+h' }\nx = { name = 'Only host', shortcut = 'cmd+x' }",
        );
        let finder = package(
            "finder",
            "com.apple.finder",
            "gh = { name = 'Home', shortcut = 'cmd+shift+h' }",
        );
        for (input, active, expected) in [("gh", false, "Host"), ("x", true, "Only host")] {
            let matches = collect_matching_chord_contexts(
                [&host, &finder],
                &event(input, "dev.fixture"),
                active,
            );
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].1.chord.name, expected);
        }
    }

    #[test]
    fn actual_finder_does_not_run_a_binding_twice() {
        let finder = package(
            "finder",
            "com.apple.finder",
            "gh = { name = 'Home', shortcut = 'cmd+shift+h' }",
        );
        let matches =
            collect_matching_chord_contexts([&finder], &event("gh", "com.apple.finder"), true);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn picker_context_cannot_override_reserved_core_namespace() {
        // Core-only leading-zero bindings must remain inaccessible to ordinary packages.
        let finder = package(
            "finder",
            "com.apple.finder",
            "gh = { name = 'Home', shortcut = 'cmd+shift+h' }",
        );
        let matches =
            collect_matching_chord_contexts([&finder], &event("0gh", "dev.fixture"), true);
        assert!(matches.is_empty());
    }
}

mod bun_handlers {
    use crate::app::AppHandleExt;
    use crate::bun_js::{package_worker, protocol::Request};
    use std::collections::HashSet;
    use tauri::AppHandle;

    pub async fn retain_runtime_registrations(
        handle: AppHandle,
        handler_ids: HashSet<String>,
    ) -> anyhow::Result<()> {
        let packages = handle.app_state().chord_package_manager();
        let roots = packages.package_roots.read().clone();
        package_worker::retain_handlers(handler_ids, roots).await;
        Ok(())
    }

    pub async fn register_handler(
        handle: AppHandle,
        module_specifier: String,
        raw: serde_json::Value,
        pathslug_string: String,
        bundle_id: Option<String>,
        build_args: Vec<toml::Value>,
    ) -> anyhow::Result<String> {
        package_worker::register(
            handle,
            module_specifier.clone(),
            Request::Register {
                module: module_specifier,
                raw,
                pathslug: pathslug_string,
                app_id: bundle_id,
                args: build_args,
            },
        )
        .await
    }
}
