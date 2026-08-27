use crate::models::FilePathslug;
use crate::quickjs::{format_js_error, with_js};
use anyhow::{Context, Result};
use llrt_core::Module;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use typeshare::typeshare;

type FullImportSpecifier = String;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordJsPackage {
    pub name: String,
    files: HashMap<FilePathslug, String>,
}

pub struct ChordJsPackageBuilder {
    name: String,
    handle: AppHandle,
}

impl ChordJsPackage {
    pub fn builder(handle: AppHandle, name: &str) -> ChordJsPackageBuilder {
        ChordJsPackageBuilder {
            name: name.to_string(),
            handle,
        }
    }

    /// Gets the module specifier for a file, supporting nested (bundled) packages.
    /// The pathslug is used to detect nested packages.
    /// "file.js" from chords/com/app/macos.toml -> "js/file.js"
    /// "file.js" from chords/@pkg/name/chords/com/app/macos.toml -> "js/@pkg/name/js/file.js"
    pub fn resolve_file(
        &self,
        chords_file_pathslug: &FilePathslug,
        file: &str,
    ) -> Result<Option<String>> {
        let path = Path::new(chords_file_pathslug.as_os_str());
        let components: Vec<_> = path.components().collect();

        // 1. Determine the JS root based on the directory structure
        let js_root = if components.len() >= 4
            && components[0].as_os_str() == "chords"
            && components[1]
                .as_os_str()
                .to_str()
                .unwrap_or("")
                .starts_with('@')
            && components[3].as_os_str() == "chords"
        {
            // Scoped case: chords/@pkg/name/chords/... -> js/@pkg/name/js
            let pkg_scope = components[1].as_os_str();
            let pkg_name = components[2].as_os_str();

            Path::new("js").join(pkg_scope).join(pkg_name).join("js")
        } else if components.first().map(|c| c.as_os_str()) == Some(std::ffi::OsStr::new("chords"))
        {
            // Standard case: chords/... -> js
            PathBuf::from("js")
        } else {
            anyhow::bail!("Path does not match a recognized chords directory structure");
        };

        // 2. Join the target file (e.g., "file.js") to our resolved root
        let final_js_path = js_root.join(file);

        // 3. Convert to string and resolve through the package registry
        let import_specifier = final_js_path
            .to_str()
            .context("Failed to convert path to UTF-8")?;

        Ok(self.resolve_import(import_specifier))
    }

    /// Gets the module specifier for a pathslug import, e.g. "js/tray.js"
    pub fn resolve_import(&self, import_specifier: &str) -> Option<String> {
        self.files
            .get(&FilePathslug::from(import_specifier))
            .map(|_| format!("{}/{}", self.name, import_specifier))
    }
}

impl ChordJsPackageBuilder {
    pub async fn load(self, files: HashMap<FilePathslug, String>) -> Result<ChordJsPackage> {
        for (file_pathslug, js) in &files {
            let file_pathslug = file_pathslug.to_owned();
            let js_string = js.clone();
            let path = PathBuf::from(self.name.clone()).join(file_pathslug.clone());
            let module_specifier = path.to_str().context("invalid path")?.to_string();
            with_js(self.handle.clone(), move |ctx| {
                Box::pin(async move {
                    async {
                        log::debug!("declaring module {}", module_specifier);
                        let module = Module::declare(
                            ctx.clone(),
                            module_specifier.clone(),
                            js_string.clone(),
                        )?;
                        let meta = module.meta()?;
                        meta.set("url", module_specifier)?;
                        let (_evaluated, promise) = module.eval()?;
                        promise.into_future::<()>().await?;
                        Ok(())
                    }
                    .await
                    .map_err(|e| anyhow::anyhow!(format_js_error(&ctx, e)))
                })
            })
            .await?;
        }

        Ok(ChordJsPackage {
            name: self.name,
            files,
        })
    }
}

#[derive(Debug)]
pub struct PackageSpecifier<'a> {
    pub package: &'a str,
    #[allow(dead_code)]
    pub subpath: Option<&'a str>,
}

impl<'a> PackageSpecifier<'a> {
    pub fn parse(specifier: &'a str) -> Self {
        if specifier.starts_with('@') {
            // Scoped package
            let mut parts = specifier.splitn(3, '/');

            match (parts.next(), parts.next(), parts.next()) {
                (Some(scope), Some(name), rest) => {
                    let pkg_len = scope.len() + 1 + name.len();
                    let package = &specifier[..pkg_len];

                    Self {
                        package,
                        subpath: rest,
                    }
                }
                _ => Self {
                    package: specifier,
                    subpath: None,
                },
            }
        } else {
            // Unscoped package
            let mut parts = specifier.splitn(2, '/');

            let package = parts.next().unwrap_or(specifier);
            let subpath = parts.next();

            Self { package, subpath }
        }
    }
}
