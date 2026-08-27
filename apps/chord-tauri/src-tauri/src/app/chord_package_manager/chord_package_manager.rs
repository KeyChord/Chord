use crate::app::AppHandleExt;
use crate::app::chord_package_manager::chord_package_registry::ChordPackageRegistry;
use crate::app::chord_package_manager::{
    ChordJsPackage, ChordNativePackage, ChordPackage, ChordReference,
};
use crate::app::state::AppSingleton;
use crate::models::{
    ChordInput, ChordInputEvent, ChordsFileHandlerKind, ChordsFileImportOverride,
    CompiledChordsFile, CompiledChordsFileHandler, FilePathslug, NATIVE_TARGET_DIR,
    NATIVE_TARGET_TRIPLE, ParsedChordsFile, RawChordPackage, RawChordsFile, is_app_sandboxed,
    toml_value_to_native_arg,
};
use chord_native_protocol::NativeHandlerRegistration;
use crate::quickjs::{format_js_error, with_js};
use crate::state::{
    ChordPackageManagerObservable, ChordPackageManagerState, GitReposObservable, Observable,
};
use anyhow::{Context, Result};
use llrt_core::libs::utils::result::ResultExt;
use nject::{inject, injectable};
use ordermap::OrderMap;
use parking_lot::{Mutex, RwLock};
use rquickjs::{Module, Object, Value, function::Args};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::task::JoinSet;
use tracing::{Instrument, info, info_span};

#[injectable]
pub struct ChordPackageManager {
    pub registry: ChordPackageRegistry,

    /// An ordered mapping from package name to package. Uses an OrderMap to allow the user to
    /// prioritize certain packages
    #[inject(RwLock::new(OrderMap::new()))]
    packages: RwLock<OrderMap<String, ChordPackage>>,

    observable: ChordPackageManagerObservable,
    handle: AppHandle,
}

struct ChordInputEventContext {
    chord_package: ChordPackage,
    event: ChordInputEvent,
}

/// A package that finished loading plus the native handler registrations it contributes to the
/// next native host generation. Registrations are kept out of `ChordPackage` because they carry
/// cache paths that have no business in frontend state.
pub struct LoadedPackage {
    pub package: ChordPackage,
    pub native_registrations: Vec<NativeHandlerRegistration>,
}

impl ChordPackageManager {
    pub async fn reload_all(&self) -> Result<()> {
        let raw_chord_packages = self.registry.import_all_packages()?;
        self.packages.write().clear();

        let mut chord_packages = Vec::new();
        let mut native_registrations = Vec::new();
        for (package_name, raw_chord_package) in raw_chord_packages {
            if let Ok(loaded) = self
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
                chord_packages.push(loaded.package);
                native_registrations.extend(loaded.native_registrations);
            };
        }

        // The native host must be fully loaded before the new package state is observable, so a
        // chord can never resolve to a handler the host cannot run.
        let supervisor = self.handle.app_state().native_host_supervisor();
        match supervisor.activate_generation(native_registrations).await {
            Ok(summary) if summary.handler_count > 0 || !summary.failed.is_empty() => {
                log::info!(
                    "activated native generation {} with {} handlers from {} libraries ({} failed)",
                    summary.generation_id,
                    summary.handler_count,
                    summary.library_count,
                    summary.failed.len()
                );
            }
            Ok(_) => {}
            Err(error) => log::error!(
                "failed to activate the native handler generation: {error:#}; native handlers are unavailable until the next reload"
            ),
        }

        self.observable.set_state(|_| ChordPackageManagerState {
            packages: chord_packages,
        })?;

        Ok(())
    }

    pub fn get_package_by_name(&self, package_name: &str) -> Option<ChordPackage> {
        self.packages.read().get(package_name).cloned()
    }

    pub async fn load_package(&self, raw_chord_package: RawChordPackage) -> Result<LoadedPackage> {
        let name = raw_chord_package.package_name();
        log::debug!("loading package {}", name);

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
            .load_js_package(&name, raw_chord_package.js_files_contents.clone())
            .await?;
        let native_package =
            self.load_native_package(&name, raw_chord_package.native_files_contents)?;

        let shared_parsed_chords_files = Arc::new(parsed_chords_files.clone());
        let shared_js_package = Arc::new(js_package.clone());
        let shared_native_package = Arc::new(native_package.clone());
        let native_registrations = Arc::new(Mutex::new(Vec::new()));

        let mut set = JoinSet::new();

        for (pathslug, parsed_chord_file) in parsed_chords_files {
            let handle = self.handle.clone();
            let parsed_chords_files = Arc::clone(&shared_parsed_chords_files);
            let js_package = Arc::clone(&shared_js_package);
            let native_package = Arc::clone(&shared_native_package);
            let native_registrations = Arc::clone(&native_registrations);
            let name = name.clone();

            // let span = info_span!("compiling_file", file = %pathslug.to_string_lossy());
            set.spawn(async move {
                let result = Self::compile_chords_file(
                    handle,
                    &parsed_chord_file,
                    pathslug.clone(),
                    &js_package,
                    &native_package,
                    &native_registrations,
                    &parsed_chords_files,
                    &None,
                )
                .await;

                // Return the data back to the main thread
                (pathslug, name, result)
            });
        }

        // 2. Collect results as they finish (Promise.all style)
        while let Some(res) = set.join_next().await {
            let (pathslug, name, compile_result) = res?;

            match compile_result {
                Ok(chords_file) => {
                    log::debug!(
                        "compiled chords file {:#?} with {} chords",
                        Path::new(&name).join(&pathslug),
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

                    compiled_chords_files.insert(pathslug.to_owned(), chords_file);
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
            native_package,
            compiled_chords_files,
            raw_chords_files,
            global_chords,
        };

        self.packages.write().insert(name, chord_package.clone());

        let native_registrations = std::mem::take(&mut *native_registrations.lock());
        Ok(LoadedPackage {
            package: chord_package,
            native_registrations,
        })
    }

    fn load_native_package(
        &self,
        package_name: &str,
        files: HashMap<FilePathslug, Vec<u8>>,
    ) -> Result<Option<ChordNativePackage>> {
        if files.is_empty() {
            return Ok(None);
        }
        log::debug!("loading native package {}", package_name);
        let cache_root = self
            .handle
            .app_state()
            .native_host_supervisor()
            .cache_dir()?;
        let package = ChordNativePackage::load(package_name, files, &cache_root)?;
        Ok(Some(package))
    }

    /// Resolves a `kind = "native"` handler to a materialized library and records the
    /// registration for the next host generation. No user code runs here.
    fn compile_native_handler(
        native_package: &Option<ChordNativePackage>,
        native_registrations: &Mutex<Vec<NativeHandlerRegistration>>,
        pathslug: &FilePathslug,
        event: &str,
        file: &str,
        args: &[toml::Value],
    ) -> Result<String> {
        anyhow::ensure!(
            !is_app_sandboxed(),
            "handler {event}: native handlers are unavailable in the sandboxed build of Chord"
        );
        let Some(native_package) = native_package else {
            anyhow::bail!(
                "handler {event}: kind = \"native\" requires prebuilt artifacts for {NATIVE_TARGET_TRIPLE} in the package's {NATIVE_TARGET_DIR}/ directory"
            );
        };
        let Some(artifact) = native_package.resolve_file(pathslug, file)? else {
            anyhow::bail!(
                "handler {event}: native library {file:?} not found in package {} (expected {:?})",
                native_package.name,
                ChordNativePackage::library_pathslug(pathslug, file)?
            );
        };
        let handler_arguments = args
            .iter()
            .map(toml_value_to_native_arg)
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("handler {event}: invalid static argument"))?;

        let handler_id = uuid::Uuid::new_v4().to_string();
        native_registrations.lock().push(NativeHandlerRegistration {
            handler_id: handler_id.clone(),
            library_path: artifact.materialized_path.clone(),
            handler_arguments,
            package_name: native_package.name.clone(),
            chords_file_pathslug: pathslug.to_string_lossy().into_owned(),
        });
        Ok(handler_id)
    }

    async fn load_js_package(
        &self,
        package_name: &str,
        files: HashMap<FilePathslug, String>,
    ) -> Result<Option<ChordJsPackage>> {
        log::debug!("loading JS package {}", package_name);

        if files.is_empty() {
            log::debug!("JS package {} was empty", package_name);
            return Ok(None);
        }

        let package = ChordJsPackage::builder(self.handle.clone(), package_name)
            .load(files)
            .await?;
        Ok(Some(package))
    }

    pub async fn compile_chords_file(
        handle: AppHandle,
        chords_file: &ParsedChordsFile,
        pathslug: FilePathslug,
        js_package: &Option<ChordJsPackage>,
        native_package: &Option<ChordNativePackage>,
        native_registrations: &Mutex<Vec<NativeHandlerRegistration>>,
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
            if handler.kind == ChordsFileHandlerKind::Native {
                let handler_id = Self::compile_native_handler(
                    native_package,
                    native_registrations,
                    &pathslug,
                    event,
                    &file,
                    &build_args,
                )?;
                handlers.push(CompiledChordsFileHandler {
                    event: event.clone(),
                    handler_id,
                    kind: ChordsFileHandlerKind::Native,
                });
                continue;
            }

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
                anyhow::bail!("handler {event}: a JS package (js/ directory) must be present when defining a JS handler")
            };
            let Some(module_specifier) = js_package.resolve_file(&pathslug, &file)? else {
                anyhow::bail!("file {} not found in js package {}", file, js_package.name);
            };

            let handler_id = with_js(handle.clone(), move |ctx| {
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
                            rquickjs_serde::to_value(ctx.clone(), raw)
                                .or_throw_msg(&ctx, "failed to parse chords file")?,
                        )?;
                        build_context.set(
                            "chordsFilePath",
                            rquickjs_serde::to_value(ctx.clone(), pathslug_string)
                                .or_throw_msg(&ctx, "failed to parse chords file pathslug")?,
                        )?;
                        build_context.set(
                            "chordsFileAppId",
                            rquickjs_serde::to_value(ctx.clone(), bundle_id)
                                .or_throw_msg(&ctx, "failed to parse chords file app ID")?,
                        )?;

                        let mut args = Args::new(ctx.clone(), build_args.len());
                        args.this(build_context)?;
                        log::debug!("calling build_handler with args {:?}", build_args);

                        let js_args = build_args
                            .into_iter()
                            .map(|value| {
                                rquickjs_serde::to_value(ctx.clone(), value)
                                    .or_throw_msg(&ctx, "failed to convert event TOML arguments")
                            })
                            .collect::<rquickjs::Result<Vec<_>>>()?;

                        for value in js_args {
                            args.push_arg(value)?;
                        }

                        let mut handler: Value = build_handler_function.call_arg(args)?;
                        if let Some(promise) = handler.as_promise().cloned() {
                            handler = promise.into_future::<Value>().await?;
                        }

                        let handler_function = handler.as_function().cloned().or_throw_msg(
                            &ctx,
                            "the default export function must return a function",
                        )?;
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
            .await?;

            handlers.push(CompiledChordsFileHandler {
                event: event.clone(),
                handler_id,
                kind: ChordsFileHandlerKind::Js,
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
                native_package,
                native_registrations,
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
