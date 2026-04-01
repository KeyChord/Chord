use crate::models::FilePathslug;
use serde::Serialize;
use std::collections::HashMap;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordJsPackage {
    exported_files: HashMap<FilePathslug, String>,
}

impl ChordJsPackage {
    pub fn new(exported_files: HashMap<FilePathslug, String>) -> Self {
        Self { exported_files }
    }

    pub fn resolve_import(&self, import_specifier: &str) -> Option<&String> {
        self.exported_files
            .get(&FilePathslug::from(import_specifier))
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
