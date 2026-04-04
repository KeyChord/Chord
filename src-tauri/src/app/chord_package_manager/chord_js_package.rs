use crate::models::FilePathslug;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use llrt_core::Module;
use typeshare::typeshare;
use crate::quickjs::{format_js_error, with_js};
use anyhow::{Context, Result};
use tauri::AppHandle;

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
    handle: AppHandle
}

impl ChordJsPackage {
    pub fn new(handle: AppHandle, name: &str) -> ChordJsPackageBuilder {
        ChordJsPackageBuilder {
            name: name.to_string(),
            handle,
        }
    }

    pub fn resolve_import(&self, import_specifier: &str) -> Option<String> {
        self.files
            .get(&FilePathslug::from(import_specifier)).map(|s| format!("{}/{}", self.name, s))
    }
}

impl ChordJsPackageBuilder {
    pub async fn load(self, files: HashMap<FilePathslug, String>) -> Result<ChordJsPackage> {
        for (file_pathslug, js) in &files {
            let file_relpath = file_pathslug.to_owned();
            let js_string = js.clone();
            let path = PathBuf::from(self.name.clone()).join(file_relpath.clone());
            let module_specifier = path.to_str().context("invalid path")?.to_string();
            with_js(self.handle.clone(), move |ctx| {
                Box::pin(async move {
                    async {
                        log::debug!("declaring module {}", module_specifier);
                        let module = Module::declare(ctx.clone(), module_specifier.clone(), js_string.clone())?;
                        let meta = module.meta()?;
                        meta.set("url", file_relpath.to_str())?;
                        let (_evaluated, promise) = module.eval()?;
                        promise.into_future::<()>().await?;
                        Ok(())
                    }.await.map_err(|e| anyhow::anyhow!(format_js_error(&ctx, e)))
                })
            })
                .await?;
        }

        Ok(ChordJsPackage {
            name: self.name,
            files
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
