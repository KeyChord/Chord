use std::collections::HashMap;
use serde::Serialize;
use crate::models::{Chord, ChordAction, ChordHint, ChordTrigger, JavascriptChordAction, ShellChordAction, ShortcutChordAction, SimulatedShortcut, ChordHintPattern};
use crate::input::Key;
use std::str::FromStr;
use regex::Regex;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChordsFile {
    pub name: String,
    pub meta: HashMap<String, String>,

    pub relpath: String,

    pub js: HashMap<String, String>,

    pub chords: Vec<Chord>,
    pub chord_hints: Vec<ChordHint>
}

impl ChordsFile {
    pub fn parse(contents: &str) -> Self {
        let value: toml::Value = toml::from_str(contents).unwrap_or_else(|_| toml::Value::Table(Default::default()));
        let table = value.as_table().expect("TOML is not a table");

        let name = table.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();

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

        let mut js_section = HashMap::new();
        if let Some(js_val) = table.get("js") {
            if let Some(t) = js_val.as_table() {
                for (k, v) in t {
                    if let Some(s) = v.as_str() {
                        js_section.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        let mut chords = Vec::new();
        let mut chord_hints = Vec::new();
        let mut index = 0;

        if let Some(chords_val) = table.get("chords") {
            if let Some(chords_table) = chords_val.as_table() {
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

                        if let Some(command_val) = val_table.get("command") {
                            if let Some(command_str) = command_val.as_str() {
                                actions.push(ChordAction::Shell(ShellChordAction { command: command_str.to_string() }));
                            }
                        }

                        for (k, v) in val_table {
                            if k.starts_with("js:") {
                                let js_key = &k[3..];
                                if let Some(module_specifier) = js_section.get(js_key) {
                                    let args = match v {
                                        toml::Value::Array(arr) => arr.clone(),
                                        _ => vec![v.clone()],
                                    };
                                    actions.push(ChordAction::Javascript(JavascriptChordAction {
                                        module_specifier: module_specifier.clone(),
                                        args,
                                    }));
                                }
                            }
                        }

                        let trigger = if key.contains('{') || key.contains('[') || key.contains('*') {
                            ChordTrigger::Pattern(Regex::new(key).unwrap_or_else(|_| Regex::new("").unwrap()))
                        } else {
                            if let Ok(keys) = Key::parse_sequence(key) {
                                ChordTrigger::Keys(keys)
                            } else {
                                ChordTrigger::Pattern(Regex::new(key).unwrap_or_else(|_| Regex::new("").unwrap()))
                            }
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
        }

        Self {
            name,
            meta,
            relpath: String::new(),
            js: js_section,
            chords,
            chord_hints,
        }
    }
}
