use crate::models::FilePathslug;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordJsPackage {
    pub name: String,
    files: HashMap<FilePathslug, String>,
    /// Where the package lives on disk. Not serialized: the frontend must not learn install
    /// paths. Bun imports modules straight from here so `import.meta.dir` is real and Node-API
    /// add-ons can be resolved beside the package.
    #[serde(skip)]
    #[typeshare(skip)]
    root: PathBuf,
}

pub struct ChordJsPackageBuilder {
    name: String,
    root: PathBuf,
    handle: AppHandle,
}

impl ChordJsPackage {
    pub fn builder(handle: AppHandle, name: &str, root: PathBuf) -> ChordJsPackageBuilder {
        ChordJsPackageBuilder {
            name: name.to_string(),
            root,
            handle,
        }
    }

    /// The package directory on disk.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Absolute paths of every JS file in the package.
    pub fn file_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.files.keys().map(|pathslug| self.root.join(pathslug))
    }

    /// Resolves a handler file to the absolute on-disk path imported by Bun.
    pub fn resolve_file_path(
        &self,
        chords_file_pathslug: &FilePathslug,
        file: &str,
    ) -> Result<Option<PathBuf>> {
        let pathslug = Self::js_pathslug(chords_file_pathslug, file)?;
        Ok(self
            .files
            .contains_key(&pathslug)
            .then(|| self.root.join(&pathslug)))
    }

    /// The pathslug of handler `file` declared by the chords file at `chords_file_pathslug`.
    fn js_pathslug(chords_file_pathslug: &FilePathslug, file: &str) -> Result<PathBuf> {
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
        Ok(js_root.join(file))
    }
}

/// Resolves a path relative to a module's *logical* package root to a path relative to the
/// on-disk package root.
///
/// A module at `js/menu.js` belongs to the package itself, so `target/x` is `target/x`. A module
/// at `js/@scope/name/js/menu.js` belongs to the vendored package `@scope/name`, whose folders
/// were copied to `js/@scope/name`, `chords/@scope/name` and `target/@scope/name` — so its
/// `target/x` is `target/@scope/name/target/x`. This is what the `chord` module's
/// `resolvePackageFile(import.meta, path)` applies.
pub fn resolve_logical_package_path(module_relpath: &Path, relative: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        relative.is_relative()
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "package-relative path {relative:?} must be relative and must not contain `..`"
    );
    let components: Vec<_> = module_relpath.components().collect();
    let vendored = components.len() >= 5
        && components[0].as_os_str() == "js"
        && components[1]
            .as_os_str()
            .to_str()
            .unwrap_or("")
            .starts_with('@')
        && components[3].as_os_str() == "js";
    if !vendored {
        return Ok(relative.to_path_buf());
    }
    let vendor = Path::new(components[1].as_os_str()).join(components[2].as_os_str());
    let mut parts = relative.components();
    let top = parts
        .next()
        .context("package-relative path must not be empty")?;
    Ok(Path::new(top.as_os_str())
        .join(vendor)
        .join(top.as_os_str())
        .join(parts.as_path()))
}

impl ChordJsPackageBuilder {
    pub async fn load(self, files: HashMap<FilePathslug, String>) -> Result<ChordJsPackage> {
        use crate::bun_js::with_js;

        let paths: Vec<String> = files
            .keys()
            .map(|pathslug| self.root.join(pathslug).to_string_lossy().into_owned())
            .collect();
        with_js(self.handle.clone(), move |ctx| {
            Box::pin(async move {
                for path in paths {
                    ctx.evict_module(&path);
                }
                Ok(())
            })
        })
        .await?;

        Ok(ChordJsPackage {
            name: self.name,
            files,
            root: self.root,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_paths_of_the_package_itself_are_unchanged() {
        let resolved = resolve_logical_package_path(
            Path::new("js/menu.js"),
            Path::new("target/t/menu/menu.node"),
        )
        .unwrap();
        assert_eq!(resolved, PathBuf::from("target/t/menu/menu.node"));
    }

    #[test]
    fn logical_paths_of_vendored_packages_are_remapped() {
        let resolved = resolve_logical_package_path(
            Path::new("js/@keychord/chords-menu/js/menu.js"),
            Path::new("target/t/menu/menu.node"),
        )
        .unwrap();
        assert_eq!(
            resolved,
            PathBuf::from("target/@keychord/chords-menu/target/t/menu/menu.node")
        );
    }

    #[test]
    fn logical_paths_cannot_escape_the_package() {
        assert!(resolve_logical_package_path(Path::new("js/menu.js"), Path::new("../x")).is_err());
        assert!(
            resolve_logical_package_path(Path::new("js/menu.js"), Path::new("/etc/passwd"))
                .is_err()
        );
    }
}
