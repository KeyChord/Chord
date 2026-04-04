use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::app::chord_package_manager::{ChordJsPackage, ChordPackage, ChordReference};
use crate::app::chord_package_registry::ChordPackageRegistry;
use crate::app::state::StateSingleton;
use crate::models::{ChordInput, ChordsFileImportOverride, CompiledChordsFile, CompiledChordsFileHandler, FilePathslug, ParsedChordsFile, RawChordPackage, RawChordsFile};
use crate::observables::{ChordPackageManagerObservable, ChordPackageManagerState, Observable};
use crate::quickjs::{format_js_error, with_js};
use anyhow::{Context, Result};
use llrt_core::{Module, Object};
use llrt_core::function::Args;
use llrt_core::libs::utils::result::ResultExt;
use ordermap::OrderMap;
use parking_lot::RwLock;
use tauri::AppHandle;

pub struct ChordPackageManager {
    // Packages is a OrderMap because we want to let the user prioritize certain packages
    packages: RwLock<OrderMap<String, ChordPackage>>,
    pub registry: ChordPackageRegistry,

    observable: ChordPackageManagerObservable,
    handle: AppHandle,
}

impl StateSingleton for ChordPackageManager {
    fn new(handle: AppHandle) -> Self {
        Self {
            packages: RwLock::new(OrderMap::new()),
            registry: ChordPackageRegistry::new(handle.clone()),
            observable: ChordPackageManagerObservable::placeholder(),
            handle,
        }
    }
}

impl ChordPackageManager {
    pub fn init(&self, observable: ChordPackageManagerObservable) -> Result<()> {
        self.observable.init(observable);
        Ok(())
    }

    pub async fn reload_all(&self) -> Result<()> {
        let raw_chord_packages = self.registry.import_all_packages()?;
        self.packages.write().clear();

        let mut chord_packages = Vec::new();
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
                chord_packages.push(package);
            };
        }

        self.observable.set_state(ChordPackageManagerState {
            packages: chord_packages,
        })?;

        Ok(())
    }

    pub fn get_package_by_name(&self, package_name: &str) -> Option<ChordPackage> {
        self.packages.read().get(package_name).cloned()
    }

    pub async fn load_package(&self, raw_chord_package: RawChordPackage) -> Result<ChordPackage> {
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

        for (pathslug, parsed_chord_file) in &parsed_chords_files {
            let Ok(chords_file) = self.compile_chords_file(
                parsed_chord_file,
                pathslug.to_owned(),
                &js_package,
                &parsed_chords_files,
                &None
            ).await
                .inspect_err(|e| {
                    log::error!(
                        "skipping chords file {:?} in {} because of compilation error: {:?}",
                        pathslug,
                        name,
                        e
                    );
                })
            else {
                continue;
            };

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

            compiled_chords_files.insert(pathslug.to_owned(), chords_file);
        }

        let chord_package = ChordPackage {
            name: name.clone(),
            js_package,
            compiled_chords_files,
            raw_chords_files,
            global_chords,
        };

        self.packages.write().insert(name, chord_package.clone());

        Ok(chord_package)
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

        let package = ChordJsPackage::new(self.handle.clone(), package_name).load(files).await?;
        Ok(Some(package))
    }

    pub async fn compile_chords_file(
        &self,
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
                        build_args.push(meta_value.clone());
                        continue;
                    }
                }

                build_args.push(arg.clone());
            }

            let file = handler.file.clone();
            let raw = chords_file.raw.clone();
            let Some(js_package) = js_package else {
                anyhow::bail!("A JS Package must be present when defining a handler")
            };

            let handler_id = with_js(self.handle.clone(), move |ctx| {
                Box::pin(async move {
                    let module_promise = Module::import(&ctx, file.to_string())?;
                    let module = module_promise.into_future::<Object>().await?;
                    let mut export: llrt_core::Value = module.get("default")?;

                    if let Some(promise) = export.as_promise().cloned() {
                        export = promise.into_future::<llrt_core::Value>().await?;
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

                    let mut handler: llrt_core::Value = build_handler_function.call_arg(args)?;
                    if let Some(promise) = handler.as_promise().cloned() {
                        handler = promise.into_future::<llrt_core::Value>().await?;
                    }

                    let handler_function = handler.as_function().cloned().or_throw_msg(
                        &ctx,
                        "the default export function must return a function",
                    )?;
                    let globals = ctx.globals();
                    let registry_key = "__RUST_HANDLER_REGISTRY";

                    // Fetch the global registry object, or create it if it doesn't exist
                    let registry: llrt_core::Object = match globals.get(registry_key) {
                        Ok(obj) => obj,
                        Err(_) => {
                            let obj = llrt_core::Object::new(ctx.clone())?;
                            globals.set(registry_key, obj.clone())?;
                            obj
                        }
                    };
                    let id = uuid::Uuid::new_v4().to_string();
                    registry.set(&id, handler_function)?;
                    Ok(id)
                })
            }).await?;

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

            let compiled_file = Box::pin(self.compile_chords_file(
                imported_file,
                imported_file_path,
                &None,
                parsed_chords_files,
                &import.r#override,
            )).await?;
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

    pub fn resolve_package_for_input(&self, input: &ChordInput) -> Option<ChordPackage> {
        let packages = self.packages.read();

        if let Some(app_id) = &input.application_id {
            let path = format!("chords/{}/macos.toml", app_id.replace(".", "/"));
            let path = PathBuf::from(path);
            for package in packages.values() {
                if let Some(chords_file) = package.compiled_chords_files.get(&path) {
                    if chords_file
                        .chords
                        .iter()
                        .find(|c| c.trigger.matches(&input.keys))
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
                .find(|c| c.chord.trigger.matches(&input.keys))
                .is_some()
            {
                return Some(package.clone());
            }
        }

        None
    }
}
