use crate::input::Key;
use crate::models::path::FilePathslug;
use crate::models::toml::TomlValue;
use crate::models::{
    Chord, ChordAction, ChordHint, ChordHintPattern, ChordTrigger, EmitChordAction,
    ShellChordAction, ShortcutChordAction, SimulatedShortcut,
};
use anyhow::Context;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{PathBuf};
use std::str::FromStr;
use llrt_core::{Object};
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

/// Currently only supports JavaScript handlers
#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileHandler {
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
    pub handler_id: String
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
        let pattern = if raw_pattern.contains('(') {
            if let Ok(re) = Regex::new(raw_pattern) {
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
            ChordTrigger::Pattern(Regex::new(key)?)
        } else {
            ChordTrigger::Keys(Key::parse_sequence(key)?)
        };

        Ok(trigger)
    }

    fn parse_actions(_key: &str, value: &Table) -> Result<Vec<ChordAction>> {
        let mut actions = Vec::new();
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
            if k.starts_with("emit:") {
                let event_key = k.strip_prefix("emit:").unwrap_or_default().to_string();
                let args = v.as_array().context("emit value must be an array")?;
                actions.push(ChordAction::Emit(EmitChordAction {
                    event_key,
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

impl FromStr for ParsedChordsFile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: toml::Value = toml::from_str(s)?;
        let root = value.as_table().context("root must be a table")?;
        let name = root
            .get("name")
            .and_then(|v| v.as_str())
            .context("the `name` property must be present")?;
        let meta = Self::parse_meta(root)?;
        let handlers = Self::parse_handlers(root)?;
        let imports = Self::parse_imports(root)?;

        let mut chords = Vec::new();
        let mut chord_hints = Vec::new();
        let mut index = 0;

        if let Some(raw_chords) = root.get("chords") {
            let raw_chords = raw_chords.as_table().context("chords must be a table")?;
            for (key, value) in raw_chords {
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

                if key.starts_with('?') {
                    if let Ok(hint) = Self::parse_hint(key, chord_value)
                        .inspect_err(|e| log::warn!("skipping hint {} because of parse error: {}", key, e))
                    {
                        chord_hints.push(hint);
                    }
                } else {
                    if let Ok(chord) = Self::parse_chord(key, chord_value, index)
                        .inspect_err(|e| log::warn!("skipping chord {} because of parse error: {}", key, e))
                    {
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
            chords,
            chord_hints,
        })
    }
}
