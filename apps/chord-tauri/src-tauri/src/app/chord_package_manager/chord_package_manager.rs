use crate::app::AppHandleExt;
use crate::app::chord_package_manager::chord_package_registry::{
    ChordPackageRegistry, ImportedPackages,
};
use crate::app::chord_package_manager::{
    ChordJsPackage, ChordPackage, ChordReference, chords_file_app_id,
};
use crate::app::state::AppSingleton;
use crate::models::{
    Chord, ChordAction, ChordEventTrigger, ChordInput, ChordInputEvent, ChordsFileImportOverride,
    CompiledChordsFile,
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

/// A package keeps what it loaded last time, worker and all, when that came from the same
/// revision of a pinned GitHub source. Official packages load through a link named after their
/// source's path, and pinned sources are directories named after the commit, so the files behind
/// the same link can't have changed. Anything else (personal, linked or third-party) loads again.
fn can_keep_loaded_package(
    root: &Path,
    previous_root: Option<&Path>,
    source: Option<&Path>,
    pinned_sources: Option<&Path>,
    worker_running: bool,
) -> bool {
    worker_running
        && previous_root == Some(root)
        && source
            .zip(pinned_sources)
            .is_some_and(|(source, sources)| source.starts_with(sources))
}

#[cfg(test)]
mod keep_loaded_package_tests {
    use super::*;

    #[test]
    fn only_an_unchanged_pinned_source_with_a_live_worker_is_kept() {
        let sources = Path::new("/u/.chord/packages/.sources/github.com");
        let pinned = sources.join("keychord/chords-menu/e6bb912e");
        let link = Path::new("/u/.chord/packages/@keychord/.managed/1f2e3d/chords-menu");
        let keep = |root: &Path, previous: Option<&Path>, source: &Path, running| {
            can_keep_loaded_package(root, previous, Some(source), Some(sources), running)
        };
        assert!(keep(link, Some(link), &pinned, true));
        assert!(!keep(link, Some(link), &pinned, false), "stopped worker");
        let other = Path::new("/u/.chord/packages/@keychord/.managed/4c5b6a/chords-menu");
        assert!(!keep(link, Some(other), &pinned, true), "another revision's link");
        assert!(!keep(link, None, &pinned, true), "not loaded before");
        let checkout = Path::new("/u/code/chords/@keychord/chords-menu");
        assert!(!keep(link, Some(link), checkout, true), "linked to a checkout");
        let personal = Path::new("/u/.chord/packages/default");
        assert!(!keep(personal, Some(personal), personal, true), "editable package");
        assert!(!can_keep_loaded_package(link, Some(link), None, Some(sources), true), "source missing");
    }
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

/// Takes out the actions that emit to a handler that failed to load, and the chords left with
/// none, so one handler that can't load (such as a native add-on built for another architecture)
/// doesn't take the rest of its file with it. Returns how many chords were dropped.
fn without_failed_handlers(chords: Vec<Chord>, failed_events: &HashSet<String>) -> (Vec<Chord>, usize) {
    if failed_events.is_empty() {
        return (chords, 0);
    }
    let before = chords.len();
    let chords = chords
        .into_iter()
        .filter_map(|mut chord| {
            let count = chord.actions.len();
            chord.actions.retain(|action| {
                !matches!(action, ChordAction::Emit(emit) if failed_events.contains(&emit.event_key))
            });
            // A chord that never had actions is the package's business, not a failed handler's.
            (chord.actions.len() == count || !chord.actions.is_empty()).then_some(chord)
        })
        .collect::<Vec<_>>();
    let dropped = before - chords.len();
    (chords, dropped)
}

#[cfg(test)]
mod handler_args_tests {
    use super::*;

    #[test]
    fn a_handler_that_fails_to_load_only_takes_the_chords_that_need_it() {
        let file: ParsedChordsFile = r#"
name = "Menu"
[on.menu]
file = "menu.js"
[on.tray]
file = "tray.js"
[chords]
'=0' = { name = "Apple menu", '@input:menu' = [] }
'=vu' = { name = "Volume Up", shortcut = "AudioVolumeUp" }
'=x' = { name = "Either", shortcut = "cmd+x", '@input:menu' = ["x"] }
'=c' = { name = "Control Center", '@input:tray' = ["CONTROL_CENTER"] }
"#
        .parse()
        .unwrap();
        let failed = HashSet::from(["menu".to_string()]);
        let (chords, dropped) = without_failed_handlers(file.chords.clone(), &failed);
        let mut names = chords.iter().map(|chord| chord.name.as_str()).collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, ["Control Center", "Either", "Volume Up"]);
        assert_eq!(dropped, 1);
        let either = chords.iter().find(|chord| chord.name == "Either").unwrap();
        assert!(matches!(either.actions.as_slice(), [ChordAction::Shortcut(_)]));

        let (all, dropped) = without_failed_handlers(file.chords.clone(), &HashSet::new());
        assert_eq!((all.len(), dropped), (file.chords.len(), 0));
    }

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
        let package = super::chord_package_registry::core::load_core_package(
            &self.registry.local.package_store()?,
        )?;
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

    /// Changes every package before a single reload, so bulk approval does not reload once per
    /// package.
    pub async fn set_package_approvals(&self, identities: &[String], approved: bool) -> Result<()> {
        let _reload_guard = self.reload_lock.lock().await;
        let available = self.observable.get_state()?.third_party_packages;
        let packages = identities
            .iter()
            .map(|identity| {
                available
                    .iter()
                    .find(|package| &package.identity == identity)
                    .context("Package is no longer available. Reload Chord and review it again.")
            })
            .collect::<Result<Vec<_>>>()?;
        let targets = packages
            .iter()
            .map(|package| (Path::new(&package.path), package.identity.as_str()))
            .collect::<Vec<_>>();
        self.registry.trust.set_approvals(&targets, approved)?;
        if !approved {
            // Revoke before reading any sources: even a failed reload must not leave the
            // package's old shortcuts, lifecycle callbacks, or supervised workers active.
            if let Err(error) = self.handle.app_state().chord_mode_manager().stop_loops() {
                // Cancellation happens before publishing the loop state; keep revoking even
                // if the UI could not be notified.
                log::warn!("Failed to publish stopped loops while revoking approval: {error:#}");
            }
            for package in &packages {
                {
                    let mut loaded = self.packages.write();
                    if let Some(mut removed) = loaded.remove(&package.name) {
                        if package.name == CORE_CHORD_PACKAGE {
                            // Keep the already compiled protected bindings without reading disk:
                            // revoking an override must still work if its sources disappeared.
                            removed.js_package = None;
                            removed
                                .compiled_chords_files
                                .retain(|path, _| is_core_chords_file(&package.name, path));
                            removed
                                .raw_chords_files
                                .retain(|path, _| is_core_chords_file(&package.name, path));
                            removed.global_chords.retain(|chord| {
                                is_core_chords_file(&package.name, &chord.chords_file_pathslug)
                            });
                            loaded.insert(removed.name.clone(), removed);
                        }
                    }
                }
                self.package_roots.write().remove(&package.name);
            }
            let retained = handler_ids(&self.packages.read());
            bun_handlers::retain_runtime_registrations(self.handle.clone(), retained).await?;
            self.update_executable_app_ids();
            self.observable.set_state(|mut state| {
                state.packages = self.packages.read().values().cloned().collect();
                for entry in &mut state.third_party_packages {
                    if identities.contains(&entry.identity) {
                        entry.approved = false;
                    }
                }
                state
            })?;
        }
        self.reload_once(&[]).await
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
            third_party: third_party_packages,
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

        // Packages earlier in the order win within each scope (see `rank_packages`).
        let personal_names = personal_packages
            .iter()
            .filter_map(|state| state.name.clone())
            .collect::<Vec<_>>();
        let saved_order = self.saved_package_order();
        let (ranked, order) = rank_packages(
            raw_chord_packages.keys().cloned(),
            &personal_names,
            &saved_order,
            &builtin_packages(),
        );
        if order != saved_order {
            // Remember where newly seen packages went, so a later install lands above them.
            if let Err(error) = self.handle.app_state().chord_package_store().set_order(order) {
                log::warn!("Failed to save the package order: {error:#}");
            }
        }
        let mut raw_chord_packages = raw_chord_packages;
        let raw_chord_packages = ranked
            .iter()
            .filter_map(|name| raw_chord_packages.remove_entry(name))
            .collect::<Vec<_>>();

        let mut chord_packages = OrderMap::new();
        let mut package_errors = HashMap::new();
        let previous_packages = self.packages.read().clone();
        let previous_errors = self
            .observable
            .get_state()
            .map(|state| state.errors)
            .unwrap_or_default();
        let pinned_sources = self
            .registry
            .git
            .git_repos_store
            .github_repos_dir()
            .ok()
            .and_then(|sources| std::fs::canonicalize(sources).ok());
        for (package_name, raw_chord_package) in raw_chord_packages {
            // Adding one pack used to restart every package's worker, which takes a while for
            // each under Rosetta.
            let kept = previous_packages.get(&package_name).filter(|_| {
                !raw_chord_package.js_files_contents.is_empty()
                    && can_keep_loaded_package(
                        &raw_chord_package.root,
                        previous_roots.get(&package_name).map(PathBuf::as_path),
                        std::fs::canonicalize(&raw_chord_package.root).ok().as_deref(),
                        pinned_sources.as_deref(),
                        crate::bun_js::package_worker::is_running(&package_name),
                    )
            });
            if let Some(package) = kept {
                log::debug!("keeping package {package_name}: same pinned revision, worker running");
                let errors = previous_errors.get(&package_name).cloned().unwrap_or_default();
                package_errors.insert(package_name, errors);
                chord_packages.insert(package.name.clone(), package.clone());
                continue;
            }
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
        // Selecting a source is distinct from trusting its executable code. Keep newly
        // selected third-party packages visible for approval without compiling them.
        let required = required_packages
            .iter()
            .filter(|name| {
                package_errors.contains_key(*name)
                    || !third_party_packages
                        .iter()
                        .any(|package| &package.name == *name && !package.approved)
            })
            .cloned()
            .collect::<Vec<_>>();
        ensure_selected_packages_loaded(&required, &package_errors)?;
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
            third_party_packages,
            builtin_packages: builtin_packages().into_iter().collect(),
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

    fn saved_package_order(&self) -> Vec<String> {
        self.handle
            .app_state()
            .chord_package_store()
            .order()
            .inspect_err(|error| log::warn!("Failed to read the package order: {error:#}"))
            .unwrap_or_default()
    }

    /// Saves the user's ranking of installed and built-in packages and applies it without a
    /// reload, so running loops and package workers carry on. Names Chord ranks on its own
    /// (core and personal packages) or doesn't know are kept but have no effect.
    pub async fn set_package_order(&self, order: Vec<String>) -> Result<()> {
        let _reload_guard = self.reload_lock.lock().await;
        let store = self.handle.app_state().chord_package_store();
        store.set_order(merge_package_order(&store.order()?, order))?;
        let state = self.observable.get_state()?;
        let personal_names = state
            .personal_packages
            .iter()
            .filter_map(|state| state.name.clone())
            .collect::<Vec<_>>();
        let saved_order = self.saved_package_order();
        let packages = {
            let mut packages = self.packages.write();
            let (ranked, _) = rank_packages(
                packages.keys().cloned(),
                &personal_names,
                &saved_order,
                &builtin_packages(),
            );
            let mut reordered = OrderMap::new();
            for name in ranked {
                if let Some(package) = packages.get(&name) {
                    reordered.insert(name, package.clone());
                }
            }
            *packages = reordered;
            packages.values().cloned().collect::<Vec<_>>()
        };
        // Executable claims follow package priority.
        self.update_executable_app_ids();
        self.observable.set_state(|mut state| {
            state.packages = packages;
            state
        })?;
        Ok(())
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
            .filter(|(_, root)| super::package_module_relative_path(root, path).is_some())
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
        anyhow::ensure!(
            self.registry.local.package_store()?.contains(&root),
            "Package {name} must load from ~/.chord/packages"
        );

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
            let mut handler_errors = Vec::new();
            let compile_result = Self::compile_chords_file(
                self.handle.clone(),
                parsed_chord_file,
                pathslug.clone(),
                &js_package,
                &parsed_chords_files,
                &None,
                &mut handler_errors,
            )
            .await;
            errors.extend(handler_errors);

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
        handler_errors: &mut Vec<String>,
    ) -> Result<CompiledChordsFile> {
        validate_chords_imports(&pathslug, parsed_chords_files)?;
        Self::compile_validated_chords_file(
            handle,
            chords_file,
            pathslug,
            js_package,
            parsed_chords_files,
            r#override,
            handler_errors,
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
        handler_errors: &mut Vec<String>,
    ) -> Result<CompiledChordsFile> {
        log::debug!("compiling chords file {}", chords_file.name);

        let mut chords = chords_file.chords.clone();
        let mut chord_hints = chords_file.chord_hints.clone();
        let mut handlers = Vec::new();
        let mut failed_events = HashSet::new();
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
            let registered = bun_handlers::register_handler(
                handle.clone(),
                module_path.to_string_lossy().into_owned(),
                raw,
                pathslug_string,
                bundle_id,
                build_args,
            )
            .await;
            match registered {
                Ok(handler_id) => handlers.push(CompiledChordsFileHandler {
                    event: event.clone(),
                    handler_id,
                }),
                // The file's other chords still work, so keep them and say what's missing.
                Err(error) => {
                    log::error!(
                        "skipping handler {event} in {} because it failed to load: {error:#}",
                        pathslug.display()
                    );
                    handler_errors.push(format!(
                        "Handler {event} in {} failed to load: {error:#}",
                        pathslug.display()
                    ));
                    failed_events.insert(event.clone());
                }
            }
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
                handler_errors,
            ))
            .await?;
            chords.extend(compiled_file.chords.clone());
            chord_hints.extend(compiled_file.chord_hints.clone());
            handlers.extend(compiled_file.handlers.clone());
        }

        // An import may have loaded a handler for the same event.
        failed_events.retain(|event| !handlers.iter().any(|handler| &handler.event == event));
        let (chords, dropped) = without_failed_handlers(chords, &failed_events);
        if dropped > 0 {
            log::warn!(
                "left out {dropped} chords in {} that need a handler that failed to load",
                pathslug.display()
            );
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

fn builtin_packages() -> HashSet<String> {
    crate::default_packages::builtin_package_names()
        .inspect_err(|error| log::error!("Failed to read Chord's default packages: {error:#}"))
        .unwrap_or_default()
        .into_iter()
        .filter(|name| name != CORE_CHORD_PACKAGE)
        .collect()
}

/// Ranks packages, highest first, without priority numbers:
///
/// 1. Chord's protected core package. No other package can use its `0` namespace anyway.
/// 2. Personal packages in `~/.chord/packages`, in folder-name order.
/// 3. Installed packages, in the user's saved order. One the order doesn't list yet, such as a
///    package installed since, goes above the others.
/// 4. Built-in packages (Chord's defaults), in the user's saved order. An installed package
///    always overrides a built-in one.
///
/// `saved` is the user's order. Returns the ranked names and the saved order with newly seen
/// packages added; names that aren't loaded right now keep their place in it.
fn rank_packages(
    names: impl IntoIterator<Item = String>,
    personal: &[String],
    saved: &[String],
    builtins: &HashSet<String>,
) -> (Vec<String>, Vec<String>) {
    let names = names.into_iter().collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let saved = saved
        .iter()
        .filter(|name| seen.insert(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let ranked_by_user = |name: &String| name != CORE_CHORD_PACKAGE && !personal.contains(name);
    let mut new = names
        .iter()
        .filter(|name| ranked_by_user(*name) && !saved.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    new.sort();
    let (new_builtins, new_installed): (Vec<_>, Vec<_>) =
        new.into_iter().partition(|name| builtins.contains(name));
    let order = new_installed
        .into_iter()
        .chain(saved)
        .chain(new_builtins)
        .collect::<Vec<_>>();

    let loaded = |name: &&String| names.contains(*name) && ranked_by_user(*name);
    let ranked = names
        .get(CORE_CHORD_PACKAGE)
        .into_iter()
        .chain(personal.iter().filter(|name| names.contains(*name)))
        .chain(order.iter().filter(loaded).filter(|name| !builtins.contains(*name)))
        .chain(order.iter().filter(loaded).filter(|name| builtins.contains(*name)))
        .cloned()
        .collect();
    (ranked, order)
}

/// Applies a new order for some packages, such as the loaded ones Settings shows, to the saved
/// order. A saved package missing from `order` stays right after the package it followed.
fn merge_package_order(saved: &[String], order: Vec<String>) -> Vec<String> {
    let given = order.iter().cloned().collect::<HashSet<_>>();
    let mut following = HashMap::<Option<String>, Vec<String>>::new();
    let mut previous = None;
    for name in saved {
        if given.contains(name) {
            previous = Some(name.clone());
        } else {
            following.entry(previous.clone()).or_default().push(name.clone());
        }
    }
    let mut merged = following.remove(&None).unwrap_or_default();
    for name in order {
        let after = following.remove(&Some(name.clone())).unwrap_or_default();
        merged.push(name);
        merged.extend(after);
    }
    merged
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
    // A fallback chord, such as a launcher's `/(.*)`, yields to every ordinary chord in every
    // scope and package. Fallbacks keep the same scope and package order among themselves.
    let (ordinary, fallbacks): (Vec<_>, Vec<_>) = contexts
        .into_iter()
        .partition(|(_, reference)| !reference.chord.fallback);
    ordinary.into_iter().chain(fallbacks).collect()
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
mod package_rank_tests {
    use super::*;

    fn rank(
        names: &[&str],
        personal: &[&str],
        saved: &[&str],
        builtins: &[&str],
    ) -> (Vec<String>, Vec<String>) {
        let owned = |names: &[&str]| names.iter().map(|name| name.to_string()).collect::<Vec<_>>();
        rank_packages(
            owned(names),
            &owned(personal),
            &owned(saved),
            &owned(builtins).into_iter().collect(),
        )
    }

    #[test]
    fn installed_packages_override_built_ins_and_follow_the_saved_order() {
        let (ranked, order) = rank(
            &["finder", "raycast", CORE_CHORD_PACKAGE, "mine", "warp", "spotlight"],
            &["mine"],
            &["spotlight", "warp", "finder", "raycast"],
            &["finder", "spotlight"],
        );
        // A built-in ranked above an installed package by the user still comes after it.
        assert_eq!(
            ranked,
            [CORE_CHORD_PACKAGE, "mine", "warp", "raycast", "spotlight", "finder"]
        );
        assert_eq!(order, ["spotlight", "warp", "finder", "raycast"]);
    }

    #[test]
    fn new_installs_go_on_top_and_new_built_ins_go_last() {
        let (ranked, order) = rank(
            &["warp", "raycast", "rift", "finder", "spotlight"],
            &[],
            &["warp", "finder", "gone"],
            &["finder", "spotlight"],
        );
        assert_eq!(ranked, ["raycast", "rift", "warp", "finder", "spotlight"]);
        // A package that isn't loaded right now keeps its place for when it returns.
        assert_eq!(order, ["raycast", "rift", "warp", "finder", "gone", "spotlight"]);

        let (ranked, again) = rank(
            &["warp", "raycast", "rift", "finder", "spotlight"],
            &[],
            &order.iter().map(String::as_str).collect::<Vec<_>>(),
            &["finder", "spotlight"],
        );
        assert_eq!(again, order, "a saved order is stable");
        assert_eq!(ranked, ["raycast", "rift", "warp", "finder", "spotlight"]);
    }

    #[test]
    fn core_and_personal_packages_are_not_ranked_by_the_user() {
        let (ranked, order) = rank(
            &["mine", CORE_CHORD_PACKAGE, "warp"],
            &["mine"],
            &["warp", "mine", "warp", CORE_CHORD_PACKAGE],
            &[],
        );
        assert_eq!(ranked, [CORE_CHORD_PACKAGE, "mine", "warp"]);
        assert_eq!(order, ["warp", "mine", CORE_CHORD_PACKAGE], "duplicates are dropped");
    }

    #[test]
    fn reordering_loaded_packages_keeps_offline_ones_beside_their_neighbors() {
        let saved = ["offline-first", "warp", "offline-a", "raycast", "offline-b"]
            .map(String::from);
        let merged = merge_package_order(&saved, vec!["raycast".into(), "warp".into()]);
        assert_eq!(merged, ["offline-first", "raycast", "offline-b", "warp", "offline-a"]);
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

    fn global_package(name: &str, bindings: &str) -> ChordPackage {
        let parsed: ParsedChordsFile = format!("name = '{name}'\n[chords]\n{bindings}")
            .parse()
            .unwrap();
        let path = PathBuf::from("chords/macos.toml");
        ChordPackage {
            name: name.into(),
            js_package: None,
            raw_chords_files: HashMap::new(),
            compiled_chords_files: HashMap::new(),
            global_chords: parsed
                .chords
                .into_iter()
                .map(|chord| ChordReference {
                    package_name: name.into(),
                    chords_file_pathslug: path.clone(),
                    chord,
                })
                .collect(),
        }
    }

    fn names(matches: &[(ChordPackage, ChordReference)]) -> Vec<&str> {
        matches.iter().map(|(_, reference)| reference.chord.name.as_str()).collect()
    }

    #[test]
    fn ordinary_chords_beat_fallbacks_from_higher_ranked_packages() {
        let raycast = global_package(
            "raycast",
            "'/(.*)' = { name = 'Raycast', shell = 'true', fallback = true }",
        );
        let spotlight = global_package(
            "spotlight",
            "'/(.*)' = { name = 'Spotlight', shell = 'true', fallback = true }",
        );
        let finder = global_package("finder", "'/f' = { name = 'Finder', shell = 'true' }");
        let host = package(
            "host",
            "dev.fixture",
            "'/(.*)' = { name = 'Host launcher', shell = 'true', fallback = true }",
        );
        let order = [&host, &raycast, &spotlight, &finder];

        let matches = collect_matching_chord_contexts(order, &event("/f", "dev.fixture"), false);
        assert_eq!(names(&matches), ["Finder", "Host launcher", "Raycast", "Spotlight"]);

        // Fallbacks keep scope and package order: the app's own launcher, then the higher rank.
        let matches = collect_matching_chord_contexts(order, &event("/calc", "dev.fixture"), false);
        assert_eq!(names(&matches), ["Host launcher", "Raycast", "Spotlight"]);
        let matches =
            collect_matching_chord_contexts(order, &event("/calc", "dev.elsewhere"), false);
        assert_eq!(names(&matches), ["Raycast", "Spotlight"]);
    }

    #[test]
    fn a_package_prefers_its_own_ordinary_chord_over_its_fallback() {
        let launcher = global_package(
            "launcher",
            "'/(.*)' = { name = 'Launch', shell = 'true', fallback = true }\n\
             '/f' = { name = 'Finder', shell = 'true' }",
        );
        let input = event("/f", "dev.fixture");
        let resolved = launcher.resolve_chord_from_input_event(&input).unwrap();
        assert_eq!(resolved.chord.name, "Finder");
        let resolved = launcher.resolve_chord_from_input_event(&event("/calc", "dev.fixture"));
        assert_eq!(resolved.unwrap().chord.name, "Launch");
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
