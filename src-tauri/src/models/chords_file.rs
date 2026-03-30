use std::collections::HashMap;
use serde::Serialize;
use crate::models::{Chord, ChordAction, ChordHint, ChordTrigger, ShellChordAction, ShortcutChordAction, SimulatedShortcut, ChordHintPattern, EmitChordAction};
use crate::input::Key;
use std::str::FromStr;
use anyhow::Context;
use regex::Regex;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFile {
    pub name: String,

    // User-defined metadata. Can be anything
    pub meta: HashMap<String, String>,

    pub relpath: String,

    pub handlers: HashMap<String, ChordsFileHandler>,

    pub imports: Vec<ChordsFileImports>,

    pub chords: Vec<Chord>,
    pub chord_hints: Vec<ChordHint>
}

/// Currently only supports JavaScript handlers
#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileHandler {
    pub file: String,
    pub args: Vec<toml::Value>
}

// New struct for imports
#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFileImports {
    pub file: String,
}


impl FromStr for ChordsFile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: toml::Value = toml::from_str(s)?;
        let table = value.as_table().context("root must be a table")?;
        let name = table.get("name").and_then(|v| v.as_str()).context("the `name` property must be present")?;

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
        }

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

        let mut imports = Vec::new();
        if let Some(import_arr_val) = table.get("import") {
            let import_array = import_arr_val.as_array().context("import must be an array")?;
            for import_val in import_array {
                let import_table = import_val.as_table().context("import item be a table")?;
                let file = import_table.get("file").and_then(|f| f.as_str()).context("import must have file key")?;
                imports.push(ChordsFileImports { file: file.to_string() });
            }
        }

        let mut chords = Vec::new();
        let mut chord_hints = Vec::new();
        let mut index = 0;

        if let Some(chords_val) = table.get("chords") {
            let chords_table = chords_val.as_table().context("chords must be a table")?;
            for (key, val) in chords_table {
                let Some(val_table) = val.as_table() else { continue; };

                let chord_name = val_table.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();

                if key.starts_with('?') {
                    let pattern_str = &key[1..];
                    let pattern = if pattern_str.contains('{') || pattern_str.contains('[') || pattern_str.contains('*') {
                        if let Ok(re) = Regex::new(pattern_str) {
                            ChordHintPattern::Regex(re)
                        } else {
                            if let Ok(keys) = Key::parse_sequence(pattern_str) {
                                ChordHintPattern::Keys(keys)
                            } else {
                                ChordHintPattern::Regex(Regex::new("").unwrap())
                            }
                        }
                    } else {
                        if let Ok(keys) = Key::parse_sequence(pattern_str) {
                            ChordHintPattern::Keys(keys)
                        } else {
                            ChordHintPattern::Regex(Regex::new(pattern_str).unwrap_or_else(|_| Regex::new("").unwrap()))
                        }
                    };

                    chord_hints.push(ChordHint {
                        pattern,
                        description: chord_name,
                    });
                } else {
                    let mut actions = Vec::new();

                    if let Some(shortcut_val) = val_table.get("shortcut") {
                        if let Some(shortcut_str) = shortcut_val.as_str() {
                            if let Ok(simulated_shortcut) = SimulatedShortcut::from_str(shortcut_str) {
                                actions.push(ChordAction::Shortcut(ShortcutChordAction { simulated_shortcut }));
                            }
                        }
                    }

                    if let Some(command_val) = val_table.get("shell") {
                        if let Some(command_str) = command_val.as_str() {
                            actions.push(ChordAction::Shell(ShellChordAction { command: command_str.to_string() }));
                        }
                    }

                    for (k, v) in val_table {
                        if k.starts_with("emit:") {
                            let event_key = k.strip_prefix("emit:").unwrap_or_default().to_string();
                            let args = match v {
                                toml::Value::Array(arr) => arr.clone(),
                                _ => vec![v.clone()],
                            };
                            actions.push(ChordAction::Emit(EmitChordAction {
                                event_key,
                                args,
                            }));
                        }
                    }

                    let trigger = if key.contains('(') {
                        ChordTrigger::Pattern(Regex::new(key).unwrap_or_else(|_| Regex::new("").unwrap()))
                    } else {
                        ChordTrigger::Keys(Key::parse_sequence(key)?)
                    };

                    chords.push(Chord {
                        string_key: key.clone(),
                        trigger,
                        name: chord_name,
                        index,
                        actions,
                    });
                    index += 1;
                }
            }
        }

        Ok(Self {
            name: name.to_string(),
            meta,
            relpath: String::new(),
            handlers,
            imports,
            chords,
            chord_hints,
        })
    }
}
