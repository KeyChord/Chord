use std::collections::HashMap;
use std::path::PathBuf;
use serde::Serialize;
use crate::models::{Chord, ChordAction, ChordHint, ChordTrigger, ShellChordAction, ShortcutChordAction, SimulatedShortcut, ChordHintPattern, EmitChordAction};
use crate::input::Key;
use std::str::FromStr;
use anyhow::Context;
use regex::Regex;
use toml::Table;
use typeshare::typeshare;
use anyhow::Result;

#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFile {
    pub name: String,

    // User-defined metadata. Can be anything
    pub meta: HashMap<String, String>,

    pub handlers: HashMap<String, ChordsFileHandler>,

    pub imports: Vec<ChordsFileImports>,

    pub chords: Vec<Chord>,
    pub chord_hints: Vec<ChordHint>,

    /// This is the object exposed to the JS handler. This maximizes compatibility so that even if
    /// our internal representation changes, a user's scripts will continue to work because it only
    /// depends on the actual TOML structure and not how we parse it.
    #[typeshare(typescript = "ChordsFileRaw")]
    pub raw: serde_json::Value
}

#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawChordsFile {
    pub name: String,
    pub meta: HashMap<String, String>,
    pub handlers: HashMap<String, ChordsFileHandler>,
    #[typeshare(typescript = "Record<string, any>")]
    pub chords: HashMap<String, toml::Value>,
    pub imports: Vec<ChordsFileImports>,
}

/// Currently only supports JavaScript handlers
#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileHandler {
    pub file: String,
    #[typeshare(typescript = "any[]")]
    pub args: Vec<toml::Value>
}

// New struct for imports
#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileImports {
    pub file: String,
}

impl ChordsFile {
    fn parse_meta(table: &Table) -> Result<HashMap<String, String>> {
        let mut meta = HashMap::new();
        if let Some(meta_val) = table.get("meta") {
            if let Some(t) = meta_val.as_table() {
                for (k, v) in t {
                    let v_str = match v {
                        toml::Value::String(s) => s.clone(),
                        _ => v.to_string().trim_matches('"').to_string(),
                    };
                    meta.insert(k.clone(), v_str);
                }
            }
        };
        Ok(meta)
    }

    fn parse_handlers(table: &Table) -> Result<HashMap<String, ChordsFileHandler>> {
        let mut handlers = HashMap::new();
        if let Some(handlers_val) = table.get("handlers") {
            let handlers_table = handlers_val.as_table().context("handlers must be a table")?;
            for (key, val) in handlers_table {
                let handler_table = val.as_table().context("handler must be a table")?;
                let file = handler_table.get("file").and_then(|v| v.as_str()).context("handler must have the file key")?;
                let mut args_vec = Vec::new();
                if let Some(args_val) = handler_table.get("args") {
                    if let Some(args_array) = args_val.as_array() {
                        args_vec = args_array.clone();
                    }
                }
                let handler = ChordsFileHandler { file: file.to_string(), args: args_vec };
                handlers.insert(key.clone(), handler);
            }
        }
        Ok(handlers)
    }

    fn parse_imports(table: &Table) -> Result<Vec<ChordsFileImports>> {
        let mut imports = Vec::new();
        if let Some(import_arr_val) = table.get("import") {
            let import_array = import_arr_val.as_array().context("import must be an array")?;
            for import_val in import_array {
                let import_table = import_val.as_table().context("import item be a table")?;
                let file = import_table.get("file").and_then(|f| f.as_str()).context("import must have file key")?;
                imports.push(ChordsFileImports { file: file.to_string() });
            }
        }
        Ok(imports)
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
                    ChordHintPattern::Regex(Regex::new("")?)
                }
            }
        } else {
            if let Ok(keys) = Key::parse_sequence(raw_pattern) {
                ChordHintPattern::Keys(keys)
            } else {
                ChordHintPattern::Regex(Regex::new(raw_pattern)?)
            }

        };

        Ok(ChordHint {
            pattern,
            raw_pattern: raw_pattern.to_string(),
            description: chord_name.unwrap_or_default().to_string(),
        })
    }

}

impl FromStr for ChordsFile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: toml::Value = toml::from_str(s)?;
        let table = value.as_table().context("root must be a table")?;
        let name = table.get("name").and_then(|v| v.as_str()).context("the `name` property must be present")?;
        let meta = Self::parse_meta(table)?;
        let handlers = Self::parse_handlers(table)?;
        let imports = Self::parse_imports(table)?;

        let mut chords = Vec::new();
        let mut chord_hints = Vec::new();
        let mut index = 0;

        if let Some(chords_val) = table.get("chords") {
            let chords_table = chords_val.as_table().context("chords must be a table")?;
            for (key, value) in chords_table {
                let table = {
                    if let Some(table) = value.as_table() {
                        table
                    } else {
                        let array = value.as_array().context("chord value mapping must be a table or an array of tables")?;
                        let first = array.first().context("chord value mapping must not be empty")?;
                        let table= first.as_table().context("must be table")?;
                        table
                    }
                };

                // hint
                if key.starts_with('?') {
                    let hint = Self::parse_hint(key, table)?;
                    chord_hints.push(hint);
                } else {
                    let raw_trigger = key;
                    let mut actions = Vec::new();

                    if let Some(shortcut) = value.get("shortcut") {
                        let shortcut = shortcut.as_str().context("shortcut property must be a string")?;
                        let simulated_shortcut = SimulatedShortcut::from_str(shortcut)?;
                    actions.push(ChordAction::Shortcut(ShortcutChordAction { simulated_shortcut }));
                    }

                    if let Some(shell) = value.get("shell") {
                        let shell = shell.as_str().context("shell property must be a string")?;
                        actions.push(ChordAction::Shell(ShellChordAction { command: shell.to_string() }));
                    }

                    for (k, v) in table {
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
                        log::warn!("couldn't find any actions for chord {:?}", table);
                    }

                    let trigger = if raw_trigger.contains('(') {
                        ChordTrigger::Pattern(Regex::new(key).unwrap_or_else(|_| Regex::new("").unwrap()))
                    } else {
                        ChordTrigger::Keys(Key::parse_sequence(key)?)
                    };

                    let chord_name = value.get("name").and_then(|n| n.as_str());
                    chords.push(Chord {
                        raw_trigger: raw_trigger.clone(),
                        trigger,
                        name: chord_name.unwrap_or_default().to_string(),
                        index,
                        actions,
                    });
                    index += 1;
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
