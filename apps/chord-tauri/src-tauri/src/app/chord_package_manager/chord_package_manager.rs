use crate::app::AppHandleExt;
use crate::app::chord_package_manager::chord_package_registry::ChordPackageRegistry;
use crate::app::chord_package_manager::{ChordJsPackage, ChordPackage, ChordReference};
use crate::app::state::AppSingleton;
use crate::models::{
    ChordInput, ChordInputEvent, ChordsFileImportOverride, CompiledChordsFile,
    CompiledChordsFileHandler, FilePathslug, ParsedChordsFile, RawChordPackage, RawChordsFile,
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

    #[inject(AtomicU64::new(0))]
    lifecycle_generation: AtomicU64,

    observable: ChordPackageManagerObservable,
    handle: AppHandle,
}

struct ChordInputEventContext {
    chord_package: ChordPackage,
    event: ChordInputEvent,
}

fn handler_ids(packages: &OrderMap<String, ChordPackage>) -> HashSet<String> {
    packages
        .values()
        .flat_map(|package| package.compiled_chords_files.values())
        .flat_map(|file| file.handlers.iter())
        .map(|handler| handler.handler_id.clone())
        .collect()
}

impl ChordPackageManager {
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
        let result = self.reload_once().await;
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

    async fn reload_once(&self) -> Result<()> {
        let raw_chord_packages = self.registry.import_all_packages()?;
        let previous_roots = self.package_roots.read().clone();
        let previous_handler_ids = handler_ids(&self.packages.read());
        let lifecycle_generation = crate::bun_js::lifecycle::begin_registration_generation();

        let next_roots = raw_chord_packages
            .values()
            .filter(|package| !package.js_files_contents.is_empty())
            .map(|package| (package.package_name(), package.root.clone()))
            .collect::<HashMap<_, _>>();
        let mut loading_roots = previous_roots.clone();
        loading_roots.extend(next_roots.clone());
        *self.package_roots.write() = loading_roots;

        let mut chord_packages = OrderMap::new();
        for (package_name, raw_chord_package) in raw_chord_packages {
            if let Ok(package) = self
                .load_package(raw_chord_package)
                .await
                .inspect_err(|error| {
                    log::error!(
                        "skipping package {} because of loading error: {:?}",
                        package_name,
                        error
                    )
                })
            {
                chord_packages.insert(package.name.clone(), package);
            };
        }

        let next_handler_ids = handler_ids(&chord_packages);
        let retained_handler_ids = previous_handler_ids
            .union(&next_handler_ids)
            .cloned()
            .collect::<HashSet<_>>();
        let observable_packages = chord_packages.values().cloned().collect();

        *self.packages.write() = chord_packages;
        *self.package_roots.write() = next_roots;
        let observable_result = self.observable.set_state(|_| ChordPackageManagerState {
            packages: observable_packages,
        });

        self.lifecycle_generation
            .store(lifecycle_generation, Ordering::SeqCst);
        if let Err(error) = bun_handlers::retain_runtime_registrations(
            self.handle.clone(),
            retained_handler_ids,
            lifecycle_generation,
        )
        .await
        {
            log::warn!("Failed to prune stale Bun registrations after reload: {error:#}");
        }

        observable_result?;
        Ok(())
    }

    pub fn get_package_by_name(&self, package_name: &str) -> Option<ChordPackage> {
        self.packages.read().get(package_name).cloned()
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

    pub async fn load_package(&self, raw_chord_package: RawChordPackage) -> Result<ChordPackage> {
        let name = raw_chord_package.package_name();
        log::debug!("loading package {}", name);
        let root = raw_chord_package.root.clone();

        let mut raw_chords_files = HashMap::new();
        let mut compiled_chords_files = HashMap::new();
        let mut global_chords = Vec::new();
        let mut parsed_chords_files = HashMap::new();

        for (path, contents) in raw_chord_package.chords_files_contents {
            let Ok(raw_chords_file) = toml::from_str::<RawChordsFile>(&contents).inspect_err(|e| {
                log::error!(
                    "error when loading package {}; failed to parse raw chords file {}:\n{}",
                    name,
                    e,
                    contents
                );
            }) else {
                continue;
            };

            raw_chords_files.insert(path.clone(), raw_chords_file);

            let Ok(parsed_chords_file) = contents.parse::<ParsedChordsFile>().inspect_err(|e| {
                log::error!(
                    "error when loading package {}; failed to parse chords file {}:\n{}",
                    name,
                    e,
                    contents
                );
            }) else {
                continue;
            };

            parsed_chords_files.insert(path, parsed_chords_file);
        }

        let js_package = self
            .load_js_package(&name, root, raw_chord_package.js_files_contents)
            .await?;

        // Bun owns one VM on one thread, so spawning one Tokio task per file only creates a
        // memory-heavy queue. Compile sequentially and let the bounded Bun worker provide the
        // ordering and backpressure.
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

                            if is_non_alphanumeric {
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
                        "skipping chords file {:?} in {} because of compilation error: {:?}",
                        pathslug,
                        name,
                        e
                    );
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

        Ok(chord_package)
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
        log::debug!("compiling chords file {}", chords_file.name);

        let mut chords = chords_file.chords.clone();
        let mut chord_hints = chords_file.chord_hints.clone();
        let mut handlers = Vec::new();
        for (event, handler) in &chords_file.handlers {
            let mut build_args = Vec::new();
            for arg in &handler.args {
                if let Some(arg) = arg.as_str() {
                    if arg.starts_with('$') {
                        let override_arg = r#override.as_ref().and_then(|v| v.meta.get(arg));
                        let meta_value = override_arg
                            .or(chords_file.meta.get(arg))
                            .context(format!("missing arg {}", arg))?;
                        // build_args.push(meta_value.clone());
                        // continue;
                    }
                }

                build_args.push(arg.clone());
            }

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

        let is_bundled_chords_file = pathslug
            .components()
            .nth(1)
            .and_then(|c| c.as_os_str().to_str())
            .map(|s| s.starts_with('@'))
            .unwrap_or(false);
        for import in &chords_file.imports {
            let imported_file_path = if is_bundled_chords_file {
                let package_name = pathslug.components().take(3).collect::<PathBuf>();
                package_name.join("chords").join(&import.file)
            } else {
                Path::new("chords").join(&import.file)
            };

            let imported_file = parsed_chords_files
                .get(&imported_file_path)
                .context(format!("import file {:?} not found", imported_file_path))?;
            log::debug!(
                "resolved import file {:?} from path {:?}",
                imported_file.name,
                imported_file_path
            );

            let compiled_file = Box::pin(Self::compile_chords_file(
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
            chords,
            chord_hints,
        })
    }

    /// Gets the chord package that is responsible for handling a specific chord input event
    pub fn create_event_context(&self, event: &ChordInputEvent) -> Option<ChordPackage> {
        let packages = self.packages.read();

        if let Some(app_id) = &event.application_id {
            let path = format!("chords/{}/macos.toml", app_id.replace(".", "/"));
            let path = PathBuf::from(path);
            for package in packages.values() {
                if let Some(chords_file) = package.compiled_chords_files.get(&path) {
                    if chords_file
                        .chords
                        .iter()
                        .find(|c| c.trigger.matches(&event.input))
                        .is_some()
                    {
                        return Some(package.clone());
                    }
                }
            }
        }

        for package in packages.values() {
            if package
                .global_chords
                .iter()
                .find(|c| c.chord.trigger.matches(&event.input))
                .is_some()
            {
                return Some(package.clone());
            }
        }

        None
    }
}

mod bun_handlers {
    use crate::bun_js::{format_js_error, lifecycle, with_js};
    use rbun::Module;
    #[allow(unused_imports)]
    use rbun::prelude::OptionExt;
    use rbun::prelude::{Args, Object, ResultExt, Value};
    use std::collections::HashSet;
    use tauri::AppHandle;

    pub async fn retain_runtime_registrations(
        handle: AppHandle,
        handler_ids: HashSet<String>,
        lifecycle_generation: u64,
    ) -> anyhow::Result<()> {
        with_js(handle, move |ctx| {
            Box::pin(async move {
                let globals = ctx.globals();
                let registry: Option<Object> = globals.get("__RUST_HANDLER_REGISTRY")?;
                if let Some(registry) = registry {
                    let keys = registry
                        .keys::<String>()
                        .collect::<rbun::Result<Vec<_>>>()?;
                    for key in keys {
                        if !handler_ids.contains(&key) {
                            registry.remove(key)?;
                        }
                    }
                }
                lifecycle::retain_registration_generation(lifecycle_generation);
                Ok(())
            })
        })
        .await
    }

    /// Import a handler module, call its default export with the build context, and register the
    /// returned handler function.
    pub async fn register_handler(
        handle: AppHandle,
        module_specifier: String,
        raw: serde_json::Value,
        pathslug_string: String,
        bundle_id: Option<String>,
        build_args: Vec<toml::Value>,
    ) -> anyhow::Result<String> {
        with_js(handle, move |ctx| {
            Box::pin(async move {
                async {
                    let module_promise = Module::import(&ctx, module_specifier)?;
                    let module = module_promise.into_future::<Object>().await?;
                    let mut export: Value = module.get("default")?;

                    if let Some(promise) = export.as_promise().cloned() {
                        export = promise.into_future::<Value>().await?;
                    }

                    let build_handler_function = export.as_function().cloned().or_throw_msg(
                        &ctx,
                        &format!(
                            "JS default export did not resolve to a function: {:?}",
                            export
                        ),
                    )?;

                    let build_context = Object::new(ctx.clone())?;
                    build_context.set(
                        "chordsFile",
                        rbun::serde::to_value(ctx.clone(), raw)
                            .or_throw_msg(&ctx, "failed to parse chords file")?,
                    )?;
                    build_context.set(
                        "chordsFilePath",
                        rbun::serde::to_value(ctx.clone(), pathslug_string)
                            .or_throw_msg(&ctx, "failed to parse chords file pathslug")?,
                    )?;
                    build_context.set(
                        "chordsFileAppId",
                        rbun::serde::to_value(ctx.clone(), bundle_id)
                            .or_throw_msg(&ctx, "failed to parse chords file app ID")?,
                    )?;

                    let mut args = Args::new(ctx.clone(), build_args.len());
                    args.this(build_context)?;
                    log::debug!("calling build_handler with args {:?} (bun)", build_args);

                    let js_args = build_args
                        .into_iter()
                        .map(|value| {
                            rbun::serde::to_value(ctx.clone(), value)
                                .or_throw_msg(&ctx, "failed to convert event TOML arguments")
                        })
                        .collect::<rbun::Result<Vec<_>>>()?;

                    for value in js_args {
                        args.push_arg(value)?;
                    }

                    let mut handler: Value = build_handler_function.call_arg(args)?;
                    if let Some(promise) = handler.as_promise().cloned() {
                        handler = promise.into_future::<Value>().await?;
                    }

                    let handler_function = handler
                        .as_function()
                        .cloned()
                        .or_throw_msg(&ctx, "the default export function must return a function")?;
                    let globals = ctx.globals();
                    let registry_key = "__RUST_HANDLER_REGISTRY";

                    // Fetch the global registry object, or create it if it doesn't exist
                    let registry: Object = match globals.get(registry_key) {
                        Ok(obj) => obj,
                        Err(_) => {
                            let obj = Object::new(ctx.clone())?;
                            globals.set(registry_key, obj.clone())?;
                            obj
                        }
                    };
                    let id = uuid::Uuid::new_v4().to_string();
                    registry.set(&id, handler_function)?;
                    Ok(id)
                }
                .await
                .map_err(|e| anyhow::anyhow!(format_js_error(&ctx, e)))
            })
        })
        .await
    }
}
