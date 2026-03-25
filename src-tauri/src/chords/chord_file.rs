use crate::chord_runner::javascript::{ChordJsArgs, ChordJsInvocation};
use crate::chords::Chord;
use crate::input::Key;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typeshare::typeshare;
use crate::chord_runner::shortcut::Shortcut;

#[derive(Debug, Serialize)]
pub struct AppChordsFile {
    pub config: Option<AppChordsFileConfig>,
    pub chords: HashMap<String, AppChordMapValue>,
    pub placeholder_chords: Vec<AppChordPlaceholder>,
    pub descriptions: HashMap<String, String>,
    pub raw_file_json: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawAppChordsFile {
    pub config: Option<AppChordsFileConfig>,
    pub chords: Option<HashMap<String, AppChordMapValue>>,
}

impl AppChordsFile {
    pub fn parse(content: &str) -> Result<Self> {
        let toml_value: toml::Value = toml::from_str(content)?;
        let json_value: serde_json::Value = serde_json::to_value(toml_value)?;

        let file = toml::from_str::<RawAppChordsFile>(content)?;
        let mut chords = HashMap::new();
        let mut placeholder_chords = Vec::new();
        let mut descriptions = HashMap::new();

        for (sequence, value) in file.chords.unwrap_or_default() {
            let Some(description_sequence) = sequence.strip_prefix('?') else {
                if let Some(placeholder) =
                    AppChordPlaceholder::parse(sequence.clone(), value.clone())
                {
                    placeholder_chords.push(placeholder);
                    continue;
                }

                chords.insert(sequence, value);
                continue;
            };

            let Some(description) = description_text_from_value(&value) else {
                log::warn!(
                    "Skipping empty description entry for sequence: {}",
                    sequence
                );
                continue;
            };

            match expand_description_sequence(description_sequence) {
                Ok(expanded_sequences) => {
                    for expanded_sequence in expanded_sequences {
                        descriptions.insert(expanded_sequence, description.clone());
                    }
                }
                Err(error) => {
                    log::warn!(
                        "Skipping invalid description sequence {}: {}",
                        sequence,
                        error
                    );
                }
            }
        }

        Ok(Self {
            config: file.config,
            chords,
            placeholder_chords,
            descriptions,
            raw_file_json: json_value,
        })
    }

    pub fn get_raw_chords(&self) -> HashMap<String, AppChordMapValue> {
        let mut raw_chords = self.chords.clone();

        for placeholder in &self.placeholder_chords {
            raw_chords.insert(
                placeholder.sequence_template.clone(),
                placeholder.value.clone(),
            );
        }

        raw_chords
    }

    pub fn get_chords_shallow(
        &self,
        placeholder_bindings: &HashMap<String, String>,
    ) -> HashMap<Vec<Key>, Chord> {
        let mut chords = HashMap::new();

        for (sequence, value) in &self.chords {
            if let Some((keys, chord)) = build_chord(sequence, value) {
                chords.insert(keys, chord);
            }
        }

        for placeholder in &self.placeholder_chords {
            let Some(sequence) = placeholder_bindings.get(&placeholder.sequence_template) else {
                continue;
            };

            let resolved_sequence = placeholder.resolved_sequence(sequence);
            let Some((keys, chord)) = build_chord(&resolved_sequence, &placeholder.value) else {
                continue;
            };

            if chords.contains_key(&keys) {
                log::warn!(
                    "Skipping placeholder chord {} because {} is already assigned",
                    placeholder.sequence_template,
                    resolved_sequence
                );
                continue;
            }

            chords.insert(keys, chord);
        }

        chords
    }

    pub fn get_descriptions_shallow(&self) -> HashMap<Vec<Key>, String> {
        let mut descriptions = HashMap::new();

        for (sequence, description) in &self.descriptions {
            let Ok(keys) = Key::parse_sequence(sequence) else {
                log::warn!("Skipping invalid description sequence: {}", sequence);
                continue;
            };

            descriptions.insert(keys, description.clone());
        }

        descriptions
    }
}

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChordsFileConfig {
    pub name: Option<String>,
    pub extends: Option<String>,
    pub js: Option<AppChordsFileConfigJs>,
}

#[typeshare]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChordsFileConfigJs {
    pub module: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppChordArgs {
    Values(Vec<toml::Value>),
    Eval(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChord {
    pub name: String,
    pub shortcut: Option<String>,
    pub shell: Option<String>,
    pub args: Option<AppChordArgs>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

impl AppChord {
    fn js_invocation(&self) -> Result<Option<ChordJsInvocation>> {
        let mut invocation = self
            .args
            .clone()
            .map(|args| ChordJsInvocation {
                export_name: "default".into(),
                args: parse_js_args(args),
            });

        for (key, value) in &self.extra {
            let Some(export_name) = key.strip_prefix("args:") else {
                continue;
            };

            let args = parse_js_args_value(key, value)?;
            let next_invocation = ChordJsInvocation {
                export_name: export_name.to_string(),
                args,
            };

            if invocation.replace(next_invocation).is_some() {
                anyhow::bail!("Multiple JS invocation targets configured");
            }
        }

        Ok(invocation)
    }
}

fn parse_js_args(args: AppChordArgs) -> ChordJsArgs {
    match args {
        AppChordArgs::Values(values) => ChordJsArgs::Values(values),
        AppChordArgs::Eval(source) => ChordJsArgs::Eval(source),
    }
}

fn parse_js_args_value(key: &str, value: &toml::Value) -> Result<ChordJsArgs> {
    match value {
        toml::Value::Array(items) => Ok(ChordJsArgs::Values(items.clone())),
        toml::Value::String(source) => Ok(ChordJsArgs::Eval(source.clone())),
        _ => anyhow::bail!("{key} must be an array or string"),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppChordMapValue {
    Single(AppChord),
    Multiple(Vec<AppChord>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChordPlaceholder {
    pub sequence_template: String,
    pub placeholder: String,
    pub sequence_prefix: String,
    pub sequence_suffix: String,
    pub value: AppChordMapValue,
}

impl AppChordPlaceholder {
    fn parse(sequence_template: String, value: AppChordMapValue) -> Option<Self> {
        let start = sequence_template.find('<')?;
        let end = sequence_template[start + 1..].find('>')? + start + 1;
        let placeholder = sequence_template[start + 1..end].trim().to_string();

        if placeholder.is_empty() {
            return None;
        }

        if sequence_template[..start].contains(['<', '>'])
            || sequence_template[end + 1..].contains(['<', '>'])
        {
            log::warn!(
                "Skipping placeholder sequence with multiple placeholders: {}",
                sequence_template
            );
            return None;
        }

        Some(Self {
            sequence_prefix: sequence_template[..start].to_string(),
            sequence_suffix: sequence_template[end + 1..].to_string(),
            sequence_template,
            placeholder,
            value,
        })
    }

    pub fn name(&self) -> Option<String> {
        description_text_from_value(&self.value)
    }

    pub fn resolved_sequence(&self, sequence: &str) -> String {
        format!(
            "{}{}{}",
            self.sequence_prefix, sequence, self.sequence_suffix
        )
    }
}

fn description_text_from_value(value: &AppChordMapValue) -> Option<String> {
    match value {
        AppChordMapValue::Single(entry) => Some(entry.name.clone()),
        AppChordMapValue::Multiple(entries) => entries.first().map(|entry| entry.name.clone()),
    }
}

fn build_chord(sequence: &str, value: &AppChordMapValue) -> Option<(Vec<Key>, Chord)> {
    let entry = match value {
        AppChordMapValue::Single(entry) => Some(entry),
        AppChordMapValue::Multiple(entries) => entries.first(),
    };

    let Some(entry) = entry else {
        log::warn!("Skipping invalid chord entry for sequence: {}", sequence);
        return None;
    };

    let Ok(keys) = Key::parse_sequence(sequence) else {
        log::warn!("Skipping invalid sequence for chord: {}", sequence);
        return None;
    };

    let Ok(shortcut) = entry
        .shortcut
        .as_ref()
        .map(|s| Shortcut::parse(s))
        .transpose()
    else {
        log::warn!("Skipping invalid shortcut for sequence: {}", sequence);
        return None;
    };

    let Ok(js) = entry.js_invocation() else {
        log::warn!(
            "Skipping invalid JS action configuration for sequence: {}",
            sequence
        );
        return None;
    };

    let chord = Chord {
        keys: keys.clone(),
        name: entry.name.clone(),
        shortcut,
        shell: entry.shell.clone(),
        js,
    };

    Some((keys, chord))
}

fn expand_description_sequence(sequence: &str) -> Result<Vec<String>> {
    let Some(start) = sequence.find('{') else {
        return Ok(vec![sequence.to_string()]);
    };

    let end = find_matching_brace(sequence, start)
        .ok_or_else(|| anyhow::anyhow!("unclosed brace expression"))?;
    let prefix = &sequence[..start];
    let inner = &sequence[start + 1..end];
    let suffix = &sequence[end + 1..];
    let variants = expand_brace_variants(inner)?;
    let suffixes = expand_description_sequence(suffix)?;
    let mut expanded = Vec::new();

    for variant in variants {
        for suffix in &suffixes {
            expanded.push(format!("{prefix}{variant}{suffix}"));
        }
    }

    Ok(expanded)
}

fn find_matching_brace(sequence: &str, start: usize) -> Option<usize> {
    let mut depth = 0;

    for (index, ch) in sequence.char_indices().skip(start) {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
}

fn expand_brace_variants(inner: &str) -> Result<Vec<String>> {
    if inner.contains(',') {
        return Ok(inner
            .split(',')
            .map(|variant| variant.to_string())
            .collect());
    }

    let Some((start, end)) = inner.split_once("..") else {
        anyhow::bail!("unsupported brace expression");
    };

    if let (Ok(start_number), Ok(end_number)) = (start.parse::<i64>(), end.parse::<i64>()) {
        let step = if start_number <= end_number { 1 } else { -1 };
        let width = start.len().max(end.len());
        let mut current = start_number;
        let mut variants = Vec::new();

        loop {
            variants.push(format!("{current:0width$}"));
            if current == end_number {
                break;
            }
            current += step;
        }

        return Ok(variants);
    }

    let mut start_chars = start.chars();
    let mut end_chars = end.chars();
    let (Some(start_char), Some(end_char)) = (start_chars.next(), end_chars.next()) else {
        anyhow::bail!("empty brace range");
    };

    if start_chars.next().is_some() || end_chars.next().is_some() {
        anyhow::bail!("unsupported brace range");
    }

    let step = if start_char <= end_char {
        1_i32
    } else {
        -1_i32
    };
    let mut current = start_char as i32;
    let end = end_char as i32;
    let mut variants = Vec::new();

    loop {
        let Some(ch) = char::from_u32(current as u32) else {
            anyhow::bail!("invalid brace range");
        };
        variants.push(ch.to_string());
        if current == end {
            break;
        }
        current += step;
    }

    Ok(variants)
}
