use super::Key;
use crate::models::path::FilePathslug;
use crate::models::toml::TomlValue;
use crate::models::{
    BuiltinChordAction, Chord, ChordAction, ChordEventTrigger, ChordHint, ChordHintPattern,
    ChordTrigger, EmitChordAction, ShellChordAction, ShortcutChordAction, SimulatedShortcut,
};
use anyhow::Context;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use toml::{Table, Value};
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParsedChordsFile {
    pub name: String,

    // User-defined metadata. Can be anything
    pub meta: HashMap<String, TomlValue>,

    // We use a Vec because we need to encode priority
    pub handlers: HashMap<String, ChordsFileHandler>,

    pub imports: Vec<ChordsFileImport>,

    /// File names of bare executables, which macOS reports without a bundle identifier, that
    /// this app's chords file applies to (see `app/desktop_app/app_identity.rs`).
    pub executables: Vec<String>,

    pub chords: Vec<Chord>,
    pub chord_hints: Vec<ChordHint>,

    /// This is the object exposed to the JS handler. This maximizes compatibility so that even if
    /// our internal representation changes, a user's scripts will continue to work because it only
    /// depends on the actual TOML structure and not how we parse it.
    #[typeshare(serialized_as = "RawChordsFile")]
    pub raw: serde_json::Value,
}

/// A chords file that has imports inlined.
#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompiledChordsFile {
    pub name: String,
    pub pathslug: FilePathslug,
    pub meta: HashMap<String, TomlValue>,
    pub handlers: Vec<CompiledChordsFileHandler>,
    pub executables: Vec<String>,
    pub chords: Vec<Chord>,
    pub chord_hints: Vec<ChordHint>,
}

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawChordsFile {
    pub name: String,
    #[serde(default)]
    pub meta: HashMap<String, TomlValue>,
    #[serde(default)]
    pub handlers: HashMap<String, ChordsFileHandler>,
    #[serde(default)]
    pub chords: HashMap<String, TomlValue>,
    #[serde(default)]
    pub imports: Vec<ChordsFileImport>,
}

/// A handler is always a JavaScript module; native code is loaded by the module itself as a
/// Node-API add-on through `process.dlopen` (see `scripting.md`).
#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileHandler {
    /// A path inside `js/` (e.g. `menu.js`).
    pub file: String,
    #[serde(default)]
    #[typeshare(typescript(type = "any[]"))]
    pub args: Vec<toml::Value>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledChordsFileHandler {
    pub event: String,
    /// Key in the JS engine's `__RUST_HANDLER_REGISTRY`.
    pub handler_id: String,
}

// New struct for imports
#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileImport {
    pub file: String,
    pub r#override: Option<ChordsFileImportOverride>,
}

/// Currently, we only support overriding meta, but might support overriding chords in the future.
#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileImportOverride {
    pub meta: HashMap<String, TomlValue>,
}

impl ParsedChordsFile {
    fn parse_meta(table: &Table) -> Result<HashMap<String, TomlValue>> {
        let mut meta = HashMap::new();
        if let Some(meta_val) = table.get("meta") {
            if let Some(t) = meta_val.as_table() {
                for (k, v) in t {
                    meta.insert(k.clone(), v.clone());
                }
            }
        };
        Ok(meta)
    }

    fn parse_handlers(table: &Table) -> Result<HashMap<String, ChordsFileHandler>> {
        let mut handlers = HashMap::new();
        if let Some(handlers_val) = table.get("on") {
            let handlers_table = handlers_val
                .as_table()
                .context("handlers must be a table")?;
            for (key, val) in handlers_table {
                let handler_table = val.as_table().context("handler must be a table")?;
                let file = handler_table
                    .get("file")
                    .and_then(|v| v.as_str())
                    .context("handler must have the file key")?;
                // `kind` predates in-process native add-ons: "js" is accepted for compatibility; the
                // former "native" (a prebuilt library run by a sidecar) is not.
                if let Some(kind_value) = handler_table.get("kind") {
                    let kind = kind_value
                        .as_str()
                        .with_context(|| format!("handler {key}: kind must be a string"))?;
                    anyhow::ensure!(
                        kind == "js",
                        "handler {key}: unsupported kind {kind:?}; handlers are JavaScript modules — load native code from the module as a Node-API add-on with `process.dlopen` (see scripting.md)"
                    );
                }
                let mut args_vec = Vec::new();
                if let Some(args_val) = handler_table.get("args") {
                    if let Some(args_array) = args_val.as_array() {
                        args_vec = args_array.clone();
                    }
                }
                let handler = ChordsFileHandler {
                    file: file.to_string(),
                    args: args_vec,
                };
                handlers.insert(key.clone(), handler);
            }
        }
        Ok(handlers)
    }

    fn parse_executables(table: &Table) -> Result<Vec<String>> {
        let Some(value) = table.get("executables") else {
            return Ok(Vec::new());
        };
        value
            .as_array()
            .context("executables must be an array of executable file names")?
            .iter()
            .map(|name| {
                let name = name
                    .as_str()
                    .context("executables must be an array of executable file names")?;
                anyhow::ensure!(
                    !name.trim().is_empty() && !name.contains('/'),
                    "executables must name files, not paths: {name:?}"
                );
                Ok(name.to_string())
            })
            .collect()
    }

    fn parse_imports(table: &Table) -> Result<Vec<ChordsFileImport>> {
        let mut imports = Vec::new();
        if let Some(import_arr_val) = table.get("import") {
            let import_array = import_arr_val
                .as_array()
                .context("import must be an array")?;
            for import_val in import_array {
                let import_table = import_val.as_table().context("import item be a table")?;
                let file = import_table
                    .get("file")
                    .and_then(|f| f.as_str())
                    .context("import must have file key")?;
                let r#override = import_table
                    .get("override")
                    .map(|v| Self::parse_override(v).context("invalid override option"))
                    .transpose()?;
                imports.push(ChordsFileImport {
                    file: file.to_string(),
                    r#override,
                });
            }
        }
        Ok(imports)
    }

    fn parse_override(value: &Value) -> Result<ChordsFileImportOverride> {
        let table = value
            .as_table()
            .context("override must be a table if present")?;
        Ok(ChordsFileImportOverride {
            meta: Self::parse_meta(table)?,
        })
    }

    fn parse_hint(key: &str, value: &Table) -> Result<ChordHint> {
        let chord_name = value.get("name").and_then(|n| n.as_str());

        let raw_pattern = &key[1..];
        // A terminal character range is a compact display hint, not a handler regex.
        let range = Regex::new(
            r#"^(?P<prefix>[^()"]*)(?:"(?P<quoted>[a-z0-9]-[a-z0-9])"|\((?P<parenthesized>[a-z0-9]-[a-z0-9])\))$"#,
        )?;
        let pattern = if let Some(captures) = range.captures(raw_pattern) {
            let bounds = captures
                .name("quoted")
                .or_else(|| captures.name("parenthesized"))
                .context("missing hint range bounds")?
                .as_str()
                .as_bytes();
            let start = bounds[0];
            let end = bounds[2];
            anyhow::ensure!(
                start <= end && start.is_ascii_digit() == end.is_ascii_digit(),
                "invalid hint range: {}",
                raw_pattern
            );
            let sequence: String = (start..=end).map(char::from).collect();
            ChordHintPattern::Range {
                prefix: Key::parse_sequence(&captures["prefix"])?,
                keys: Key::parse_sequence(&sequence)?,
            }
        } else if raw_pattern.contains('(') {
            if let Ok(re) = parse_chord_pattern(raw_pattern) {
                ChordHintPattern::Regex(re)
            } else {
                if let Ok(keys) = Key::parse_sequence(raw_pattern) {
                    ChordHintPattern::Keys(keys)
                } else {
                    anyhow::bail!("invalid key sequence: {}", raw_pattern)
                }
            }
        } else {
            if let Ok(keys) = Key::parse_sequence(raw_pattern) {
                ChordHintPattern::Keys(keys)
            } else {
                anyhow::bail!("invalid key sequence: {}", raw_pattern)
            }
        };

        Ok(ChordHint {
            pattern,
            raw_pattern: raw_pattern.to_string(),
            description: chord_name.unwrap_or_default().to_string(),
        })
    }

    fn parse_trigger(key: &str, _value: &Table) -> Result<ChordTrigger> {
        let raw_trigger = key;

        let trigger = if raw_trigger.contains('(') {
            ChordTrigger::Pattern(parse_chord_pattern(key)?)
        } else {
            ChordTrigger::Keys(Key::parse_sequence(key)?)
        };

        Ok(trigger)
    }

    fn parse_actions(_key: &str, value: &Table) -> Result<Vec<ChordAction>> {
        let mut actions = Vec::new();
        if let Some(builtin) = value.get("builtin") {
            let action: BuiltinChordAction = builtin
                .clone()
                .try_into()
                .context("invalid built-in chord action")?;
            actions.push(ChordAction::Builtin(action));
        }
        if let Some(shortcut) = value.get("shortcut") {
            let shortcut = shortcut
                .as_str()
                .context("shortcut property must be a string")?;
            let simulated_shortcut = SimulatedShortcut::from_str(shortcut)?;
            actions.push(ChordAction::Shortcut(ShortcutChordAction {
                simulated_shortcut,
            }));
        }

        if let Some(shell) = value.get("shell") {
            let shell = shell.as_str().context("shell property must be a string")?;
            actions.push(ChordAction::Shell(ShellChordAction {
                command: shell.to_string(),
            }));
        }

        for (k, v) in value {
            if k.starts_with('@') {
                let Some((trigger, target)) = k[1..].split_once(':') else {
                    anyhow::bail!("invalid event specifier {k:?}; expected @trigger:event");
                };
                let trigger = match trigger {
                    "input" => ChordEventTrigger::Input,
                    "init" => ChordEventTrigger::Init,
                    "keydown" => ChordEventTrigger::Keydown,
                    "keyup" => ChordEventTrigger::Keyup,
                    "keypress" => ChordEventTrigger::Keypress,
                    _ => anyhow::bail!("unsupported event trigger in {k:?}"),
                };
                let (event_key, assign_to) = match target.split_once('=') {
                    Some((event, identifier)) => {
                        anyhow::ensure!(
                            is_event_identifier(identifier),
                            "invalid return-value identifier in {k:?}"
                        );
                        (event, Some(identifier.to_string()))
                    }
                    None => (target, None),
                };
                anyhow::ensure!(!event_key.is_empty(), "missing event name in {k:?}");
                let args = v
                    .as_array()
                    .with_context(|| format!("{k} value must be an array"))?;
                actions.push(ChordAction::Emit(EmitChordAction {
                    trigger,
                    event_key: event_key.to_string(),
                    assign_to,
                    args: args.clone(),
                }));
            }
        }

        if actions.is_empty() {
            log::warn!("couldn't find any actions for chord {:?}", value);
        }

        Ok(actions)
    }

    fn parse_chord(key: &str, chord: &Table, index: u32) -> Result<Chord> {
        let raw_trigger = key;
        let actions = Self::parse_actions(key, chord)?;
        let trigger = Self::parse_trigger(key, chord)?;
        let chord_name = chord.get("name").and_then(|n| n.as_str());
        Ok(Chord {
            raw_trigger: raw_trigger.to_string(),
            trigger,
            name: chord_name.unwrap_or_default().to_string(),
            index,
            actions,
        })
    }
}

/// Only parenthesized sections are regex; the surrounding chord keys are literal.
fn parse_chord_pattern(raw: &str) -> Result<Regex> {
    let mut pattern = String::from(r"\A(?:");
    let mut remaining = raw;
    while let Some(start) = remaining.find('(') {
        Key::validate_sequence_escapes(&remaining[..start])?;
        pattern.push_str(&regex::escape(&remaining[..start]));
        remaining = &remaining[start..];
        // Let the regex parser find the end of a complete group. This also handles
        // nested groups, escaped parentheses, character classes and regex comments.
        let end = remaining
            .char_indices()
            .filter(|(_, c)| *c == ')')
            .map(|(index, _)| index + 1)
            .find(|&end| Regex::new(&remaining[..end]).is_ok())
            .with_context(|| format!("invalid parenthesized regex in chord {raw:?}"))?;
        // Scope inline regex flags to this section without adding a capture group.
        pattern.push_str("(?:");
        pattern.push_str(&remaining[..end]);
        pattern.push(')');
        remaining = &remaining[end..];
    }
    Key::validate_sequence_escapes(remaining)?;
    pattern.push_str(&regex::escape(remaining));
    pattern.push_str(r")\z");
    Ok(Regex::new(&pattern)?)
}

pub(crate) fn is_event_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl FromStr for ParsedChordsFile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s, false)
    }
}

impl ParsedChordsFile {
    /// Only the registry's protected core file may opt into the meta namespace.
    pub(crate) fn parse_core(s: &str) -> Result<Self> {
        let parsed = Self::parse(s, true)?;
        anyhow::ensure!(
            parsed.handlers.is_empty() && parsed.imports.is_empty(),
            "the protected core file must be self-contained"
        );
        Ok(parsed)
    }

    fn parse(s: &str, core: bool) -> Result<Self> {
        let value: toml::Value = toml::from_str(s)?;
        let root = value.as_table().context("root must be a table")?;
        let name = root
            .get("name")
            .and_then(|v| v.as_str())
            .context("the `name` property must be present")?;
        let meta = Self::parse_meta(root)?;
        let handlers = Self::parse_handlers(root)?;
        let imports = Self::parse_imports(root)?;
        let executables = Self::parse_executables(root)?;

        let mut chords = Vec::new();
        let mut chord_hints = Vec::new();
        let mut index = 0;

        if let Some(raw_chords) = root.get("chords") {
            let raw_chords = raw_chords.as_table().context("chords must be a table")?;
            for (key, value) in raw_chords {
                anyhow::ensure!(
                    core || !key.trim_start_matches('?').starts_with('0'),
                    "the 0 prefix is reserved for built-in Chord commands: {key}"
                );
                let chord_value = {
                    if let Some(table) = value.as_table() {
                        table
                    } else {
                        let array = value
                            .as_array()
                            .context("chord value mapping must be a table or an array of tables")?;
                        let first = array
                            .first()
                            .context("chord value mapping must not be empty")?;
                        let table = first.as_table().context("must be table")?;
                        table
                    }
                };

                anyhow::ensure!(
                    core || !chord_value.contains_key("builtin"),
                    "built-in actions are reserved for Chord's protected core file"
                );
                if key.starts_with('?') {
                    if let Ok(hint) = Self::parse_hint(key, chord_value).inspect_err(|e| {
                        log::warn!("skipping hint {} because of parse error: {}", key, e)
                    }) {
                        chord_hints.push(hint);
                    }
                } else {
                    if core {
                        // Display hints also describe repeat counts and global keys.
                        // Only executable core bindings belong to the 0 namespace.
                        anyhow::ensure!(
                            key.starts_with('0'),
                            "core Chord bindings must use the 0 prefix"
                        );
                        let chord = Self::parse_chord(key, chord_value, index)?;
                        anyhow::ensure!(
                            chord.actions.len() == 1
                                && matches!(chord.actions[0], ChordAction::Builtin(_)),
                            "core Chord bindings must have one built-in action"
                        );
                        chords.push(chord);
                        index += 1;
                        continue;
                    }
                    if let Ok(chord) = Self::parse_chord(key, chord_value, index).inspect_err(|e| {
                        log::warn!("skipping chord {} because of parse error: {}", key, e)
                    }) {
                        chords.push(chord);
                    }
                    index += 1
                }
            }
        }

        let raw = serde_json::to_value(value.clone())?;

        log::debug!("finished parsing chord file with name '{}'", name);

        Ok(Self {
            name: name.to_string(),
            raw,
            meta,
            handlers,
            imports,
            executables,
            chords,
            chord_hints,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_files_declare_bare_executables_by_file_name() {
        let file: ParsedChordsFile = "name = 'LiveSplit One'\nexecutables = ['LiveSplit One']"
            .parse()
            .unwrap();
        assert_eq!(file.executables, ["LiveSplit One"]);
        let file: ParsedChordsFile = "name = 'none'".parse().unwrap();
        assert!(file.executables.is_empty());
        for invalid in [
            "executables = 'LiveSplit One'",
            "executables = [1]",
            "executables = ['']",
            "executables = ['/Applications/LiveSplit One']",
        ] {
            assert!(
                format!("name = 'invalid'\n{invalid}")
                    .parse::<ParsedChordsFile>()
                    .is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn core_hints_can_describe_repeat_counts_and_global_keys() {
        let file = ParsedChordsFile::parse_core(
            r#"name = 'fixture'
[chords]
'?"1-9"' = { name = 'Repeat count' }
'?/' = { name = 'Global' }
'?0' = { name = 'Chord' }
'0s' = { builtin = { command = 'settings' } }
"#,
        )
        .unwrap();
        assert_eq!(file.chords.len(), 1);
        assert_eq!(file.chord_hints.len(), 3);
        assert!(file.chord_hints.iter().any(|hint| {
            hint.description == "Repeat count"
                && matches!(
                    &hint.pattern,
                    ChordHintPattern::Range { prefix, keys }
                        if prefix.is_empty() && keys == &Key::parse_sequence("123456789").unwrap()
                )
        }));
        for (sequence, description) in [("/", "Global"), ("0", "Chord")] {
            assert!(file.chord_hints.iter().any(|hint| {
                hint.description == description
                    && matches!(
                        &hint.pattern,
                        ChordHintPattern::Keys(keys) if keys == &Key::parse_sequence(sequence).unwrap()
                    )
            }));
        }
    }

    #[test]
    fn core_commands_must_use_the_zero_prefix() {
        for key in ["s", "1s", "(0)s"] {
            let source = format!(
                "name = 'fixture'\n[chords]\n'{key}' = {{ builtin = {{ command = 'settings' }} }}"
            );
            let error = ParsedChordsFile::parse_core(&source).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("core Chord bindings must use the 0 prefix")
            );
        }
    }

    #[test]
    fn user_definitions_cannot_claim_zero_commands_or_hints() {
        for key in ["0s", "0h(\\d+)", "0custom", "?0", "?0s"] {
            let source = format!("name = 'fixture'\n[chords]\n'{key}' = {{ shell = 'true' }}");
            let error = source.parse::<ParsedChordsFile>().unwrap_err();
            assert!(error.to_string().contains("0 prefix is reserved"));
        }
        let file: ParsedChordsFile = "name = 'fixture'\n[chords]\n'(.*)' = { shell = 'true' }"
            .parse()
            .unwrap();
        assert!(
            !file.chords[0]
                .trigger
                .matches(&Key::parse_sequence("0s").unwrap())
        );
        assert!(
            file.chords[0]
                .trigger
                .matches(&Key::parse_sequence("ab").unwrap())
        );
        let error = "name = 'user'\n[chords]\nab = { builtin = { command = 'settings' } }"
            .parse::<ParsedChordsFile>()
            .unwrap_err();
        assert!(error.to_string().contains("built-in actions are reserved"));
    }

    #[test]
    fn arrow_notation_matches_physical_keys_in_literals_and_patterns() {
        let source = r#"name = 'fixture'
[chords]
'a\up' = { shortcut = 'up' }
'a\left(\d+)' = { shell = 'true' }
"#;
        let file: ParsedChordsFile = source.parse().unwrap();
        for input in [r"a\up", r"a\left12"] {
            assert!(
                file.chords
                    .iter()
                    .any(|chord| chord.trigger.matches(&Key::parse_sequence(input).unwrap()))
            );
        }
    }

    #[test]
    fn bare_backslash_hints_and_legacy_chords_are_skipped() {
        let file: ParsedChordsFile = r#"name = 'fixture'
[chords]
'?\' = { name = 'System Actions' }
'?\s' = { name = 'Settings' }
'?\(1-9)' = { name = 'Legacy range' }
'\s' = { shell = 'true' }
'\s(\d+)' = { shell = 'true' }
'a(\d+)\' = { shell = 'true' }
'?\up' = { name = 'Move up' }
'a\up' = { shortcut = 'up' }
'a\left(\d+)' = { shell = 'true' }
"#
        .parse()
        .unwrap();
        assert_eq!(file.chord_hints.len(), 1);
        assert_eq!(file.chord_hints[0].description, "Move up");
        assert!(matches!(
            &file.chord_hints[0].pattern,
            ChordHintPattern::Keys(keys) if keys == &Key::parse_sequence(r"\up").unwrap()
        ));
        assert_eq!(file.chords.len(), 2);
        for input in [r"a\up", r"a\left12"] {
            assert!(
                file.chords
                    .iter()
                    .any(|chord| { chord.trigger.matches(&Key::parse_sequence(input).unwrap()) })
            );
        }
    }

    #[test]
    fn regex_sections_keep_surrounding_keys_literal_and_match_the_full_sequence() {
        for (pattern, input, rejected) in [
            (r"hs.(\d+)", "hs.12", "hs,12"),
            (r"t.(\d+)", "t.12", "t,12"),
            (r".(.+)", ".b20l.5", "xb20l.5"),
            (r"/d.(\d+).", "/d.12.", "/d,12,"),
            (r"/d.,(\d+)", "/d.,12", "/dx,12"),
            (r"[(\d+)]", "[12]", "12"),
            (r"a(\d+)?", "a12?", "a12"),
            (r"a(\d+)|b", "a12|b", "b"),
        ] {
            let ChordTrigger::Pattern(regex) =
                ParsedChordsFile::parse_trigger(pattern, &Table::new()).unwrap()
            else {
                panic!("expected regex for {pattern}");
            };
            assert!(regex.is_match(input), "{pattern} should match {input}");
            for rejected in [rejected.to_string(), format!("x{input}")] {
                assert!(!regex.is_match(&rejected), "{pattern} matched {rejected}");
            }
            // A trailing character is valid when the group deliberately accepts anything.
            if pattern != r".(.+)" {
                assert!(!regex.is_match(&format!("{input}x")));
            }
        }
        let hint = ParsedChordsFile::parse_hint(r"?hs.(\d+)", &Table::new()).unwrap();
        let ChordHintPattern::Regex(regex) = hint.pattern else {
            panic!("expected regex hint");
        };
        assert!(regex.is_match("hs.12"));
        assert!(!regex.is_match("hs,12"));
    }

    #[test]
    fn regex_sections_preserve_nested_groups_classes_escapes_and_capture_numbers() {
        let regex = parse_chord_pattern(r"hs.((?:\d+|[()]+)\.?).([a-z]+)").unwrap();
        let captures = regex.captures("hs.12..reply").unwrap();
        assert_eq!(&captures[1], "12.");
        assert_eq!(&captures[2], "reply");
        assert!(regex.is_match("hs.().reply"));
        assert!(
            parse_chord_pattern(r"hs.(\(\)).(\d+)")
                .unwrap()
                .is_match("hs.().12")
        );
        assert!(
            parse_chord_pattern(r"hs.([]()]+)")
                .unwrap()
                .is_match("hs.]()")
        );
        assert!(
            parse_chord_pattern(r"hs.([a-z&&[^aeiou]]+)")
                .unwrap()
                .is_match("hs.bcd")
        );
    }

    #[test]
    fn regex_flags_are_scoped_and_invalid_sections_are_rejected() {
        let regex = parse_chord_pattern(r"hs.((?i)reply).end").unwrap();
        assert!(regex.is_match("hs.REPLY.end"));
        assert!(!regex.is_match("HS.REPLY.end"));
        assert!(!regex.is_match("hs.REPLY.END"));
        let regex = parse_chord_pattern("hs.((?x)a # ignored ( )\n b).end").unwrap();
        assert!(regex.is_match("hs.ab.end"));
        for pattern in ["hs.(", "hs.(a", "hs.([)", "hs.(a{2,1})"] {
            assert!(parse_chord_pattern(pattern).is_err(), "accepted {pattern}");
        }
    }

    #[test]
    fn toml_story_keys_match_literals_and_keep_capture_arguments() {
        let file: ParsedChordsFile = r#"
name = "Instagram"
[chords]
'hs.(\d+)' = { '@input:instagram' = ['story', '$1'] }
'hsr(\d+)' = { '@input:instagram' = ['story', '$1'] }
"#
        .parse()
        .unwrap();
        assert_eq!(file.chords.len(), 2);
        for (pattern, input) in [(r"hs.(\d+)", "hs.12"), (r"hsr(\d+)", "hsr12")] {
            let chord = file
                .chords
                .iter()
                .find(|chord| chord.raw_trigger == pattern)
                .unwrap();
            assert!(chord.trigger.matches(&Key::parse_sequence(input).unwrap()));
            assert!(
                !chord
                    .trigger
                    .matches(&Key::parse_sequence("hs,12").unwrap())
            );
            let ChordTrigger::Pattern(regex) = &chord.trigger else {
                panic!("expected regex");
            };
            assert_eq!(&regex.captures(input).unwrap()[1], "12");
            let ChordAction::Emit(action) = &chord.actions[0] else {
                panic!("expected input action");
            };
            assert_eq!(action.args[1].as_str(), Some("$1"));
        }
    }

    #[test]
    fn parses_input_actions() {
        let file: ParsedChordsFile = r#"
name = "Input actions"
[on.click]
file = "click.js"
[chords]
'j,(.+)' = { name = "Click", '@input:click' = ['$1'] }
be = { name = "Beep", '@input:beep' = [] }
mc = { name = "Menu", '@input:menu' = ['by-path', ['Window', 'Columns']] }
"#
        .parse()
        .unwrap();

        assert_eq!(file.handlers["click"].file, "click.js");
        assert_eq!(file.chords.len(), 3);
        for (trigger, event, args) in [
            ("j,(.+)", "click", toml::Value::Array(vec!["$1".into()])),
            ("be", "beep", toml::Value::Array(vec![])),
            (
                "mc",
                "menu",
                toml::Value::Array(vec![
                    "by-path".into(),
                    toml::Value::Array(vec!["Window".into(), "Columns".into()]),
                ]),
            ),
        ] {
            let chord = file
                .chords
                .iter()
                .find(|chord| chord.raw_trigger == trigger)
                .unwrap();
            let [ChordAction::Emit(action)] = chord.actions.as_slice() else {
                panic!("expected one input action for {trigger}");
            };
            assert_eq!(action.event_key, event);
            assert_eq!(action.trigger, ChordEventTrigger::Input);
            assert_eq!(action.assign_to, None);
            assert_eq!(&action.args, args.as_array().unwrap());
        }
        assert_eq!(file.raw["chords"]["j,(.+)"]["@input:click"][0], "$1");
    }

    #[test]
    fn input_actions_require_array_arguments() {
        for value in ["'click'", "1", "{ x = 1 }"] {
            let table: Table = format!("'@input:click' = {value}").parse().unwrap();
            let error = ParsedChordsFile::parse_actions("jc", &table).unwrap_err();
            assert_eq!(error.to_string(), "@input:click value must be an array");
        }
    }

    #[test]
    fn parses_all_event_triggers_and_return_assignments() {
        for (specifier, trigger) in [
            ("@input:click=result", ChordEventTrigger::Input),
            ("@init:click=result", ChordEventTrigger::Init),
            ("@keydown:click=result", ChordEventTrigger::Keydown),
            ("@keyup:click=result", ChordEventTrigger::Keyup),
            ("@keypress:click=result", ChordEventTrigger::Keypress),
        ] {
            let table: Table = format!("'{specifier}' = ['$result', '$1']")
                .parse()
                .unwrap();
            let actions = ParsedChordsFile::parse_actions("j,(.*)", &table).unwrap();
            let [ChordAction::Emit(action)] = actions.as_slice() else {
                panic!("expected event action")
            };
            assert_eq!(action.trigger, trigger);
            assert_eq!(action.event_key, "click");
            assert_eq!(action.assign_to.as_deref(), Some("result"));
            assert_eq!(action.args[0].as_str(), Some("$result"));
        }
        let table: Table = "'@init:@fixture/pkg:prepare=_context2' = []"
            .parse()
            .unwrap();
        let actions = ParsedChordsFile::parse_actions("j,(.*)", &table).unwrap();
        let ChordAction::Emit(action) = &actions[0] else {
            panic!("expected event action")
        };
        assert_eq!(action.event_key, "@fixture/pkg:prepare");
        assert_eq!(action.assign_to.as_deref(), Some("_context2"));
    }

    #[test]
    fn rejects_invalid_event_specifiers() {
        for specifier in [
            "@keydown",
            "@unknown:click",
            "@input:",
            "@init:click=",
            "@keydown:click=1",
            "@keypress:click=a-b",
            "@keyup:click=a=b",
        ] {
            let table: Table = format!("'{specifier}' = []").parse().unwrap();
            let error = ParsedChordsFile::parse_actions("j,(.*)", &table).unwrap_err();
            assert!(error.to_string().contains(specifier));
        }
        for trigger in ["init", "keydown", "keyup", "keypress"] {
            let table: Table = format!("'@{trigger}:click' = 'invalid'").parse().unwrap();
            assert!(
                ParsedChordsFile::parse_actions("j,(.*)", &table)
                    .unwrap_err()
                    .to_string()
                    .contains("must be an array")
            );
        }
    }

    #[test]
    fn parses_compact_help_ranges() {
        for (pattern, prefix, count) in [
            ("?-(a-z)", "-", 26),
            ("?m(0-9)", "m", 10),
            ("?(a-a)", "", 1),
            (r#"?"0-9""#, "", 10),
            (r#"?"1-9""#, "", 9),
            (r#"?m"0-9""#, "m", 10),
            (r#"?-"a-z""#, "-", 26),
            (r#"?\up"2-4""#, r"\up", 3),
        ] {
            let hint = ParsedChordsFile::parse_hint(pattern, &Table::new()).unwrap();
            let ChordHintPattern::Range {
                prefix: actual_prefix,
                keys,
            } = &hint.pattern
            else {
                panic!("expected range for {pattern}");
            };
            assert_eq!(actual_prefix, &Key::parse_sequence(prefix).unwrap());
            assert_eq!(keys.len(), count);
            let serialized = serde_json::to_value(&hint).unwrap();
            assert_eq!(
                serialized["pattern"]["range"]["keys"]
                    .as_array()
                    .unwrap()
                    .len(),
                count
            );
        }
        for pattern in ["?-(z-a)", "?-(0-z)", r#"?"9-0""#, r#"?m"0-z""#] {
            assert!(ParsedChordsFile::parse_hint(pattern, &Table::new()).is_err());
        }
        assert!(matches!(
            ParsedChordsFile::parse_hint("?-(.*)", &Table::new())
                .unwrap()
                .pattern,
            ChordHintPattern::Regex(_)
        ));
        assert!(matches!(
            ParsedChordsFile::parse_hint("?-a", &Table::new())
                .unwrap()
                .pattern,
            ChordHintPattern::Keys(_)
        ));
    }

    #[test]
    fn quoted_help_ranges_in_toml_are_display_only() {
        let file: ParsedChordsFile = r#"name = 'fixture'
[chords]
'?"1-9"' = { name = 'Repeat count' }
'?m"0-9"' = { name = 'Menu item number' }
'm(\d+)' = { shell = 'true' }
"#
        .parse()
        .unwrap();
        assert_eq!(file.chord_hints.len(), 2);
        assert_eq!(file.chords.len(), 1);
        for hint in &file.chord_hints {
            let ChordHintPattern::Range { prefix, keys } = &hint.pattern else {
                panic!("expected quoted range");
            };
            let digits = if prefix.is_empty() {
                "123456789"
            } else {
                "0123456789"
            };
            assert_eq!(keys, &Key::parse_sequence(digits).unwrap());
        }
        assert!(
            file.chords[0]
                .trigger
                .matches(&Key::parse_sequence("m42").unwrap())
        );
    }

    #[test]
    fn handler_kind_js_is_accepted() {
        let file: ParsedChordsFile = r#"
name = "t"
[on.menu]
file = "menu.js"
[on.other]
kind = "js"
file = "other.js"
args = ["a", 1]
"#
        .parse()
        .unwrap();
        assert_eq!(file.handlers["menu"].file, "menu.js");
        assert_eq!(file.handlers["other"].file, "other.js");
        assert_eq!(file.handlers["other"].args.len(), 2);

        // `RawChordsFile` is the serde view (key `handlers`), used for the JS `this.chordsFile`.
        let raw: RawChordsFile = toml::from_str(
            r#"
name = "t"
[handlers.menu]
file = "menu.js"
"#,
        )
        .unwrap();
        assert_eq!(raw.handlers["menu"].file, "menu.js");
    }

    #[test]
    fn native_handler_kind_is_rejected_with_guidance() {
        let error = r#"
name = "t"
[on.menu]
kind = "native"
file = "menu"
"#
        .parse::<ParsedChordsFile>()
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("handler menu"), "{message}");
        assert!(message.contains("process.dlopen"), "{message}");
    }

    #[test]
    fn unknown_handler_kind_names_the_handler() {
        let error = r#"
name = "t"
[on.menu]
kind = "swift"
file = "menu"
"#
        .parse::<ParsedChordsFile>()
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("handler menu"), "{message}");
        assert!(message.contains("swift"), "{message}");
    }
}
