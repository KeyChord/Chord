use super::PackageStore;
use crate::app::chord_package_manager::ChordPackage;
use crate::models::{
    BuiltinChordKind, CORE_CHORD_FILE, CORE_CHORD_PACKAGE, ChordAction, ChordHintPattern,
    ChordTrigger, Key, MetaChordCommand, RawChordPackage,
};
use crate::state::MetaChordHint;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const BUNDLED_BINDINGS: &str = include_str!(concat!(env!("OUT_DIR"), "/chords-chord.macos.toml"));

fn core_bindings(store: &PackageStore) -> Result<(PathBuf, String)> {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    BUNDLED_BINDINGS.hash(&mut hash);
    if cfg!(debug_assertions) {
        env!("CHORD_CORE_SOURCE").hash(&mut hash);
    }
    let root = store.root.join(".core").join(format!(
        "{}-{:016x}-{}",
        env!("CARGO_PKG_VERSION"),
        hash.finish(),
        if cfg!(debug_assertions) {
            "dev"
        } else {
            "release"
        }
    ));
    let file = root.join(CORE_CHORD_FILE);
    std::fs::create_dir_all(file.parent().context("Core file has no parent")?)?;
    let manifest = root.join("package.json");
    let manifest_contents = serde_json::json!({"name": CORE_CHORD_PACKAGE}).to_string();
    if std::fs::read_to_string(&manifest).ok().as_deref() != Some(&manifest_contents) {
        std::fs::write(manifest, manifest_contents)?;
    }
    let mut linked = false;
    #[cfg(debug_assertions)]
    {
        let source = Path::new(env!("CHORD_CORE_SOURCE"));
        if source.is_file() {
            super::package_store::ensure_link(source, &file)?;
            linked = true;
        }
    }
    if !linked && std::fs::read_to_string(&file).ok().as_deref() != Some(BUNDLED_BINDINGS) {
        let temporary = file.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        std::fs::write(&temporary, BUNDLED_BINDINGS)?;
        if let Err(error) = std::fs::rename(&temporary, &file) {
            let _ = std::fs::remove_file(&temporary);
            return Err(error.into());
        }
    }
    let mut package = store.import_package(&root)?;
    let contents = package
        .chords_files_contents
        .remove(Path::new(CORE_CHORD_FILE))
        .context("Failed to read protected Chord bindings from the package store")?;
    Ok((root, contents))
}

/// Restore the core file after every source tier, including same-name local overrides.
pub(super) fn protect_core_package(
    packages: &mut HashMap<String, RawChordPackage>,
    store: &PackageStore,
) -> Result<()> {
    let (root, contents) = core_bindings(store)?;
    protect_core_contents(packages, contents, &root);
    Ok(())
}

/// Bootstrap core controls before optional repositories or JavaScript workers load.
pub(in crate::app::chord_package_manager) fn load_core_package(
    store: &PackageStore,
) -> Result<ChordPackage> {
    compile_core_contents(&core_bindings(store)?.1)
}

fn compile_core_contents(contents: &str) -> Result<ChordPackage> {
    use crate::app::chord_package_manager::ChordReference;
    use crate::models::{CompiledChordsFile, ParsedChordsFile};
    let parsed = ParsedChordsFile::parse_core(contents)?;
    let path = PathBuf::from(CORE_CHORD_FILE);
    let global_chords = parsed
        .chords
        .iter()
        .map(|chord| ChordReference {
            package_name: CORE_CHORD_PACKAGE.into(),
            chords_file_pathslug: path.clone(),
            chord: chord.clone(),
        })
        .collect();
    let file = CompiledChordsFile {
        name: parsed.name,
        pathslug: path.clone(),
        meta: parsed.meta,
        handlers: vec![],
        executables: parsed.executables,
        chords: parsed.chords,
        chord_hints: parsed.chord_hints,
    };
    let package = ChordPackage {
        name: CORE_CHORD_PACKAGE.into(),
        js_package: None,
        global_chords,
        raw_chords_files: HashMap::from([(path.clone(), toml::from_str(contents)?)]),
        compiled_chords_files: HashMap::from([(path, file)]),
    };
    meta_configuration(&package)?;
    Ok(package)
}

fn protect_core_contents(
    packages: &mut HashMap<String, RawChordPackage>,
    contents: String,
    root: &Path,
) {
    let core = packages
        .entry(CORE_CHORD_PACKAGE.into())
        .or_insert_with(|| RawChordPackage {
            dirname: "chords-chord".into(),
            root: root.to_path_buf(),
            package_json_contents: Some(
                serde_json::json!({"name": CORE_CHORD_PACKAGE}).to_string(),
            ),
            chords_files_contents: HashMap::new(),
            js_files_contents: HashMap::new(),
            bin_files_contents: HashMap::new(),
        });
    core.chords_files_contents
        .insert(CORE_CHORD_FILE.into(), contents);
}

pub(super) fn ensure_core_selected(
    available: &HashMap<String, RawChordPackage>,
    selected: &[String],
) -> Result<()> {
    anyhow::ensure!(
        !available.contains_key(CORE_CHORD_PACKAGE)
            || selected.iter().any(|name| name == CORE_CHORD_PACKAGE),
        "Chord's core package is required and cannot be disabled"
    );
    Ok(())
}

pub(in crate::app::chord_package_manager) fn meta_configuration(
    package: &ChordPackage,
) -> Result<(Vec<MetaChordHint>, String)> {
    let file = package
        .compiled_chords_files
        .get(Path::new(CORE_CHORD_FILE))
        .context("Chord's protected core file must remain loaded")?;
    let mut hints = Vec::new();
    for hint in &file.chord_hints {
        if let ChordHintPattern::Keys(keys) = &hint.pattern {
            hints.push(MetaChordHint {
                sequence: Key::serialize_sequence(keys).context("invalid core hint")?,
                name: hint.description.clone(),
            });
        }
    }
    let mut required = std::collections::HashSet::new();
    let mut halt_template = None;
    for chord in &file.chords {
        let Some(ChordAction::Builtin(action)) = chord.actions.first() else {
            continue;
        };
        required.insert(action.command);
        if let ChordTrigger::Keys(keys) = &chord.trigger {
            hints.push(MetaChordHint {
                sequence: Key::serialize_sequence(keys).context("invalid core binding")?,
                name: chord.name.clone(),
            });
        }
        if action.command == BuiltinChordKind::HaltLoop {
            let start = chord
                .raw_trigger
                .find('(')
                .context("halt-loop requires a numeric pattern")?;
            let end = chord
                .raw_trigger
                .rfind(')')
                .context("invalid halt-loop pattern")?;
            let template = format!(
                "{}{{id}}{}",
                &chord.raw_trigger[..start],
                &chord.raw_trigger[end + 1..]
            );
            for id in [1, 12, u32::MAX] {
                let input = Key::parse_sequence(&template.replace("{id}", &id.to_string()))?;
                anyhow::ensure!(
                    action.resolve(&chord.trigger, &input)? == MetaChordCommand::Halt(id),
                    "halt-loop must capture its numeric ID"
                );
            }
            halt_template = Some(template);
        }
    }
    anyhow::ensure!(
        [
            BuiltinChordKind::Settings,
            BuiltinChordKind::Reload,
            BuiltinChordKind::Quit,
            BuiltinChordKind::HaltLoop,
            BuiltinChordKind::SetBuffer,
            BuiltinChordKind::SetMacro,
            BuiltinChordKind::SetLayout,
            BuiltinChordKind::CycleLayout,
        ]
        .iter()
        .all(|kind| required.contains(kind)),
        "Chord's core file must retain Settings, Reload, Quit, Halt, macro, and layout actions"
    );
    Ok((
        hints,
        halt_template.context("Chord's core file must provide a halt-loop binding")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChordAction, ChordLayout, Key, MetaChordCommand, ParsedChordsFile};

    #[test]
    fn bootstrap_and_protected_core_read_from_the_package_store() {
        let root = std::env::temp_dir().join(format!("chord-core-store-{}", uuid::Uuid::new_v4()));
        let store = PackageStore::new(root.join("packages"));
        let (path, contents) = core_bindings(&store).unwrap();
        assert!(store.contains(&path));
        assert_eq!(
            std::fs::read_to_string(path.join(CORE_CHORD_FILE)).unwrap(),
            contents
        );
        assert_eq!(core_bindings(&store).unwrap(), (path, contents));
        load_core_package(&store).unwrap();
        let mut packages = HashMap::new();
        protect_core_package(&mut packages, &store).unwrap();
        assert!(store.contains(&packages[CORE_CHORD_PACKAGE].root));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundled_file_defines_actions_and_matches_the_canonical_distribution_copy() {
        let distribution = include_str!(
            "../../../../../data/chords-chord/chords/com/leonsilicon/chord/macos.toml"
        );
        assert_eq!(
            BUNDLED_BINDINGS, distribution,
            "refresh the distribution copy after editing the canonical core file"
        );
        let file = ParsedChordsFile::parse_core(BUNDLED_BINDINGS).unwrap();
        for (input, expected) in [
            ("0,", MetaChordCommand::Settings),
            ("0,r", MetaChordCommand::Reload),
            ("0q", MetaChordCommand::Quit),
            ("0h12", MetaChordCommand::Halt(12)),
            ("0ls", MetaChordCommand::SetLayout(ChordLayout::SidebarLeft)),
            (
                "0lsl",
                MetaChordCommand::SetLayout(ChordLayout::SidebarLeft),
            ),
            (
                "0lf",
                MetaChordCommand::SetLayout(ChordLayout::FloatingBottom),
            ),
            (
                "0lfb",
                MetaChordCommand::SetLayout(ChordLayout::FloatingBottom),
            ),
            ("0lv", MetaChordCommand::CycleLayout),
        ] {
            let keys = Key::parse_sequence(input).unwrap();
            let chord = file
                .chords
                .iter()
                .find(|chord| chord.trigger.matches_in_namespace(&keys, true))
                .unwrap();
            let ChordAction::Builtin(action) = &chord.actions[0] else {
                panic!()
            };
            assert_eq!(action.resolve(&chord.trigger, &keys).unwrap(), expected);
        }
        for input in ["0l", "0lss", "0lfx", "0lvs"] {
            let keys = Key::parse_sequence(input).unwrap();
            assert!(
                !file
                    .chords
                    .iter()
                    .any(|chord| chord.trigger.matches_in_namespace(&keys, true))
            );
        }
    }

    #[test]
    fn core_file_is_present_without_sources_and_cannot_be_overridden_or_deselected() {
        let mut packages = HashMap::new();
        protect_core_contents(
            &mut packages,
            BUNDLED_BINDINGS.into(),
            Path::new("/test/packages/.core"),
        );
        assert!(ensure_core_selected(&packages, &[]).is_err());
        ensure_core_selected(&packages, &[CORE_CHORD_PACKAGE.into()]).unwrap();
        let core = packages.get_mut(CORE_CHORD_PACKAGE).unwrap();
        core.chords_files_contents
            .insert(CORE_CHORD_FILE.into(), "fake user settings".into());
        core.chords_files_contents
            .insert("chords/other.toml".into(), "ordinary chords".into());
        protect_core_contents(
            &mut packages,
            BUNDLED_BINDINGS.into(),
            Path::new("/test/packages/.core"),
        );
        assert_eq!(
            packages[CORE_CHORD_PACKAGE].chords_files_contents[Path::new(CORE_CHORD_FILE)],
            BUNDLED_BINDINGS
        );
        assert_eq!(
            packages[CORE_CHORD_PACKAGE].chords_files_contents[Path::new("chords/other.toml")],
            "ordinary chords"
        );
        packages.clear();
        protect_core_contents(
            &mut packages,
            BUNDLED_BINDINGS.into(),
            Path::new("/test/packages/.core"),
        );
        assert!(packages.contains_key(CORE_CHORD_PACKAGE));
    }

    #[test]
    fn core_bootstrap_and_help_follow_toml_bindings_without_optional_sources() {
        let source = BUNDLED_BINDINGS
            .replace("'0,'", "'0x'")
            .replace("0h", "0j")
            .replace("Chord: Settings'", "Preferences'");
        let mut package = compile_core_contents(&source).unwrap();
        let (hints, halt_template) = meta_configuration(&package).unwrap();
        assert_eq!(halt_template, "0j{id}");
        assert!(
            hints
                .iter()
                .any(|hint| hint.sequence == "0x" && hint.name == "Preferences")
        );
        assert!(!hints.iter().any(|hint| hint.sequence == "0,"));
        for app in [None, Some("com.apple.finder".into())] {
            let event = crate::models::ChordInputEvent {
                input: Key::parse_sequence("0x").unwrap(),
                application_id: app,
                ..Default::default()
            };
            assert_eq!(package.matching_global_chords(&event).len(), 1);
        }
        package.compiled_chords_files.clear();
        assert!(meta_configuration(&package).is_err());
        let missing_action = BUNDLED_BINDINGS.replace(
            "'0q' = { name = 'Chord: Quit', builtin = { command = 'quit' } }",
            "",
        );
        assert!(compile_core_contents(&missing_action).is_err());
    }
}
