use std::collections::HashMap;
use std::sync::RwLock;
use crate::app::SafeAppHandle;
use crate::quickjs::{with_js, reset_js};
use anyhow::Result;
use fast_radix_trie::StringRadixMap;
use llrt_core::Module;

/// A registry of all the loaded JavaScript packages (from the `js/` folder of chord packages)
pub struct ChordJsPackageRegistry {
    /// The key is the name of the chord package (e.g. "@keychord/chords-menu")
    packages: RwLock<HashMap<String, ChordJsPackage>>,

    handle: SafeAppHandle
}

#[derive(Clone)]
pub struct ChordJsPackage {
    /// The keys are just the file relpaths, e.g. `js/menu.js`
    exported_files: StringRadixMap<String>
}

#[derive(Debug)]
pub struct PackageSpecifier<'a> {
    pub package: &'a str,
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

impl ChordJsPackage {
    pub fn new(exported_files: StringRadixMap<String>) -> Self {
        Self { exported_files }
    }

    pub fn resolve_import(&self, import_specifier: &str) -> Option<&String> {
        self.exported_files.get(import_specifier)
    }
}

impl ChordJsPackageRegistry {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle, packages: RwLock::new(HashMap::new()) }
    }

    pub fn get_package_by_name(&self, package_name: &str) -> Option<ChordJsPackage> {
        self.packages.read().unwrap().get(package_name).cloned()
    }

    pub async fn load_package(&self, package_name: &str, files: Vec<(String, String)>) -> Result<()> {
        let handle = self.handle.try_handle()?;
        let mut exported_files = StringRadixMap::new();

        for (file_relpath, js) in files {
            let package_name_bytes = package_name.as_bytes().to_vec();
            exported_files.insert(file_relpath.clone(), js.clone());
            with_js(handle.clone(), move |ctx| {
                Box::pin(async move {
                    let module = Module::declare(ctx.clone(), package_name_bytes, js)?;
                    let meta = module.meta()?;
                    meta.set("url", file_relpath)?;
                    let (_evaluated, promise) = module.eval()?;
                    promise.into_future::<()>().await?;
                    Ok(())
                })
            })
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        };

        self.packages.write().unwrap().insert(package_name.to_string(), ChordJsPackage { exported_files });

        Ok(())
    }

    pub async fn reload_all_packages(&self) -> Result<()> {
        let handle = self.handle.try_handle()?;
        reset_js(handle.clone()).await?;

        let packages = self.packages.read().unwrap().clone();

        for (package_name, package) in packages {
            for (file_relpath, js) in package.exported_files {
                let package_name_bytes = package_name.as_bytes().to_vec();
                with_js(handle.clone(), move |ctx| {
                    Box::pin(async move {
                        let module = Module::declare(ctx.clone(), package_name_bytes, js)?;
                        let meta = module.meta()?;
                        meta.set("url", file_relpath)?;
                        let (_evaluated, promise) = module.eval()?;
                        promise.into_future::<()>().await?;
                        Ok(())
                    })
                })
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
            }
        }

        Ok(())
    }
}

fn get_package_name(specifier: &str) -> &str {
    if specifier.starts_with('@') {
        // Scoped: @scope/name/...
        let mut parts = specifier.splitn(3, '/');
        match (parts.next(), parts.next()) {
            (Some(scope), Some(name)) => {
                // return "@scope/name"
                let len = scope.len() + 1 + name.len();
                &specifier[..len]
            }
            _ => specifier, // fallback if malformed
        }
    } else {
        // Unscoped: name/...
        specifier.split('/').next().unwrap_or(specifier)
    }
}
