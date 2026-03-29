use fast_radix_trie::StringRadixMap;

#[derive(Clone)]
pub struct ChordJsPackage {
    /// The keys are just the file relpaths, e.g. `js/menu.js`
    exported_files: StringRadixMap<String>
}

impl ChordJsPackage {
    pub fn new(exported_files: StringRadixMap<String>) -> Self {
        Self { exported_files }
    }

    pub fn resolve_import(&self, import_specifier: &str) -> Option<&String> {
        self.exported_files.get(import_specifier)
    }
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
