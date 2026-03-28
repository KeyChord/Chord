use crate::app::chord_package::Chord;
use crate::app::chord_runner::javascript::{ChordJsArgs, ChordJsInvocation};
use crate::app::chord_runner::shortcut::Shortcut;
use crate::input::Key;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml_edit::{DocumentMut, Item};

#[derive(Debug, Serialize)]
pub struct AppChordsFile {
    pub name: Option<String>,
    pub meta: Option<HashMap<String, toml::Value>>,
    pub js: Option<HashMap<String, String>>,
    pub chords: HashMap<String, AppChordMapValue>,
    #[serde(skip_serializing)]
    pub regex_chords: Vec<AppChordRegex>,
    pub placeholder_chords: Vec<AppChordPlaceholder>,
    pub descriptions: HashMap<String, String>,
    pub raw_file_json: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAppChordsFile {
    pub name: Option<String>,
    pub meta: Option<HashMap<String, toml::Value>>,
    pub js: Option<HashMap<String, String>>,
    pub chords: Option<HashMap<String, AppChordMapValue>>,
}

impl AppChordsFile {
    pub fn parse(content: &str) -> Result<Self> {
        let chord_indexes = chord_entry_indexes(content)?;
        let toml_value: toml::Value = toml::from_str(content)?;
        let mut json_value: serde_json::Value = serde_json::to_value(toml_value)?;
        apply_raw_chord_indexes(&mut json_value, &chord_indexes);

        let mut file = toml::from_str::<RawAppChordsFile>(content)?;
        if let Some(chords) = file.chords.as_mut() {
            apply_chord_indexes(chords, &chord_indexes);
        }
        let mut chords = HashMap::new();
        let mut regex_chords = Vec::new();
        let mut placeholder_chords = Vec::new();
        let mut descriptions = HashMap::new();

        for (sequence, value) in file.chords.unwrap_or_default() {
            value.validate(&sequence)?;

            let Some(description_sequence) = sequence.strip_prefix('?') else {
                if let Some(placeholder) =
                    AppChordPlaceholder::parse(sequence.clone(), value.clone())
                {
                    placeholder_chords.push(placeholder);
                    continue;
                }

                if is_regex_sequence_template(&sequence) {
                    regex_chords.push(AppChordRegex {
                        sequence_template: sequence,
                        value,
                    });
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
            name: file.name,
            meta: file.meta,
            js: file.js,
            chords,
            regex_chords,
            placeholder_chords,
            descriptions,
            raw_file_json: json_value,
        })
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

    pub fn get_regex_chords_shallow(&self) -> Vec<AppChordRegex> {
        self.regex_chords.clone()
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChord {
    #[serde(default)]
    pub index: u32,
    pub name: String,
    pub shortcut: Option<String>,
    pub shell: Option<String>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

impl AppChord {
    fn validate(&self) -> Result<()> {
        let mut invocation_key = None::<&str>;

        for (key, value) in &self.extra {
            if key == "args" || key.starts_with("args:") {
                anyhow::bail!("`{key}` is no longer supported; use `js:<name>`");
            }

            if key == "js" {
                anyhow::bail!("`js` is not supported; use `js:<name>`");
            }

            let Some(export_name) = key.strip_prefix("js:") else {
                continue;
            };

            if export_name.is_empty() {
                anyhow::bail!("`js:<name>` must include a target name");
            }

            parse_js_args_value(key, value)?;

            if let Some(previous_key) = invocation_key.replace(key.as_str()) {
                anyhow::bail!(
                    "Multiple JS invocation targets configured: `{previous_key}` and `{key}`"
                );
            }
        }

        Ok(())
    }

    fn js_invocation(&self) -> Result<Option<ChordJsInvocation>> {
        let mut invocation = None;

        for (key, value) in &self.extra {
            let Some(export_name) = key.strip_prefix("js:") else {
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

    fn with_capture_values(&self, capture_values: &[String]) -> Self {
        let mut next = self.clone();
        next.name = substitute_capture_values_in_string(&next.name, capture_values);

        for (key, value) in &mut next.extra {
            if key.starts_with("js:") {
                substitute_capture_values_in_toml_value(value, capture_values);
            }
        }

        next
    }
}

fn parse_js_args_value(key: &str, value: &toml::Value) -> Result<ChordJsArgs> {
    match value {
        toml::Value::Array(items) => Ok(ChordJsArgs::Values(items.clone())),
        toml::Value::String(source) => Ok(ChordJsArgs::Eval(source.clone())),
        _ => anyhow::bail!("{key} must be an array or string"),
    }
}

fn chord_entry_indexes(content: &str) -> Result<HashMap<String, u32>> {
    let document = content.parse::<DocumentMut>()?;
    let Some(chords_table) = document.get("chords").and_then(Item::as_table_like) else {
        return Ok(HashMap::new());
    };

    Ok(chords_table
        .iter()
        .enumerate()
        .map(|(index, (sequence, _))| (sequence.to_string(), index as u32))
        .collect())
}

fn apply_chord_indexes(
    chords: &mut HashMap<String, AppChordMapValue>,
    chord_indexes: &HashMap<String, u32>,
) {
    for (sequence, value) in chords {
        let Some(index) = chord_indexes.get(sequence) else {
            continue;
        };

        value.set_index(*index);
    }
}

fn apply_raw_chord_indexes(
    raw_file_json: &mut serde_json::Value,
    chord_indexes: &HashMap<String, u32>,
) {
    let Some(chords) = raw_file_json
        .get_mut("chords")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };

    for (sequence, index) in chord_indexes {
        let Some(raw_value) = chords.get_mut(sequence) else {
            continue;
        };

        apply_raw_chord_index(raw_value, *index);
    }
}

fn apply_raw_chord_index(raw_value: &mut serde_json::Value, index: u32) {
    match raw_value {
        serde_json::Value::Object(object) => {
            object.insert("index".into(), serde_json::Value::from(index));
        }
        serde_json::Value::Array(items) => {
            for item in items {
                let Some(object) = item.as_object_mut() else {
                    continue;
                };

                object.insert("index".into(), serde_json::Value::from(index));
            }
        }
        _ => {}
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppChordMapValue {
    Single(AppChord),
    Multiple(Vec<AppChord>),
}

impl AppChordMapValue {
    fn validate(&self, sequence: &str) -> Result<()> {
        match self {
            Self::Single(entry) => entry
                .validate()
                .map_err(|error| anyhow::anyhow!("invalid chord `{sequence}`: {error}")),
            Self::Multiple(entries) => {
                for entry in entries {
                    entry
                        .validate()
                        .map_err(|error| anyhow::anyhow!("invalid chord `{sequence}`: {error}"))?;
                }

                Ok(())
            }
        }
    }

    fn set_index(&mut self, index: u32) {
        match self {
            Self::Single(entry) => {
                entry.index = index;
            }
            Self::Multiple(entries) => {
                for entry in entries {
                    entry.index = index;
                }
            }
        }
    }

    fn first_entry(&self) -> Option<&AppChord> {
        match self {
            Self::Single(entry) => Some(entry),
            Self::Multiple(entries) => entries.first(),
        }
    }

    pub(crate) fn build_chord_for_keys(
        &self,
        keys: Vec<Key>,
        capture_values: &[String],
        sequence_label: &str,
    ) -> Option<Chord> {
        let Some(entry) = self.first_entry() else {
            log::warn!(
                "Skipping invalid chord entry for sequence: {}",
                sequence_label
            );
            return None;
        };

        let entry = entry.with_capture_values(capture_values);

        let Ok(shortcut) = entry
            .shortcut
            .as_ref()
            .map(|s| Shortcut::parse(s))
            .transpose()
        else {
            log::warn!("Skipping invalid shortcut for sequence: {}", sequence_label);
            return None;
        };

        let Ok(js) = entry.js_invocation() else {
            log::warn!(
                "Skipping invalid JS action configuration for sequence: {}",
                sequence_label
            );
            return None;
        };

        Some(Chord {
            keys,
            index: entry.index,
            name: entry.name.clone(),
            shortcut,
            shell: entry.shell.clone(),
            js,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AppChordRegex {
    pub sequence_template: String,
    pub value: AppChordMapValue,
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
    let Ok(keys) = Key::parse_sequence(sequence) else {
        log::warn!("Skipping invalid sequence for chord: {}", sequence);
        return None;
    };

    let chord = value.build_chord_for_keys(keys.clone(), &[], sequence)?;

    Some((keys, chord))
}

fn is_regex_sequence_template(sequence: &str) -> bool {
    sequence.contains('(') && sequence.contains(')')
}

fn substitute_capture_values_in_toml_value(value: &mut toml::Value, capture_values: &[String]) {
    match value {
        toml::Value::String(source) => {
            *source = substitute_capture_values_in_string(source, capture_values);
        }
        toml::Value::Array(items) => {
            for item in items {
                substitute_capture_values_in_toml_value(item, capture_values);
            }
        }
        toml::Value::Table(table) => {
            for (_, value) in table.iter_mut() {
                substitute_capture_values_in_toml_value(value, capture_values);
            }
        }
        _ => {}
    }
}

fn substitute_capture_values_in_string(template: &str, capture_values: &[String]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }

        let mut digits = String::new();
        while chars.peek().is_some_and(|next| next.is_ascii_digit()) {
            digits.push(chars.next().expect("peeked digit must exist"));
        }

        if digits.is_empty() {
            output.push('$');
            continue;
        }

        let replacement = digits
            .parse::<usize>()
            .ok()
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| capture_values.get(index));

        if let Some(replacement) = replacement {
            output.push_str(replacement);
        }
    }

    output
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

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(value: &AppChordMapValue) -> u32 {
        match value {
            AppChordMapValue::Single(entry) => entry.index,
            AppChordMapValue::Multiple(entries) => {
                entries.first().map(|entry| entry.index).unwrap_or(0)
            }
        }
    }

    #[test]
    fn parse_assigns_indexes_in_toml_order() {
        let file = AppChordsFile::parse(
            r#"
[chords]
b = { name = "Bravo" }
'?a' = { name = "Alpha Section" }
a = [{ name = "Alpha One" }, { name = "Alpha Two" }]
'<digit>' = { name = "Digit Placeholder" }
"#,
        )
        .expect("parse should succeed");

        let AppChordMapValue::Single(bravo) =
            file.chords.get("b").expect("b chord should be present")
        else {
            panic!("b should parse as a single chord");
        };
        assert_eq!(bravo.index, 0);

        assert_eq!(file.raw_file_json["chords"]["?a"]["index"], 1);

        let AppChordMapValue::Multiple(alpha_variants) =
            file.chords.get("a").expect("a chord should be present")
        else {
            panic!("a should parse as multiple chords");
        };
        assert_eq!(alpha_variants[0].index, 2);
        assert_eq!(alpha_variants[1].index, 2);

        let placeholder = file
            .placeholder_chords
            .first()
            .expect("placeholder should be present");
        assert_eq!(index_of(&placeholder.value), 3);

        let resolved = file.get_chords_shallow(&HashMap::new());
        let keys = Key::parse_sequence("a").expect("a should be a valid sequence");
        assert_eq!(
            resolved
                .get(&keys)
                .expect("resolved chord should exist")
                .index,
            2
        );
    }

    #[test]
    fn parse_allows_files_without_top_level_config() {
        let file = AppChordsFile::parse(
            r#"
[chords]
a = { name = "Alpha" }
"#,
        )
        .expect("parse should succeed without config");

        assert!(file.name.is_none());
        assert!(file.meta.is_none());
        assert!(file.js.is_none());
        assert!(file.chords.contains_key("a"));
    }

    #[test]
    fn parse_reads_top_level_name_meta_and_js_fields() {
        let file = AppChordsFile::parse(
            r#"
name = "Alpha"

[meta]
category = "system"
rank = 1

[js]
default = "dist/default.runtime.js"
menu = "dist/menu.runtime.js"

[chords]
a = { name = "Alpha Action" }
"#,
        )
        .expect("parse should succeed");

        assert_eq!(file.name.as_deref(), Some("Alpha"));
        assert_eq!(
            file.meta
                .as_ref()
                .and_then(|meta| meta.get("category"))
                .and_then(toml::Value::as_str),
            Some("system")
        );
        assert_eq!(
            file.js
                .as_ref()
                .and_then(|js| js.get("default"))
                .map(String::as_str),
            Some("dist/default.runtime.js")
        );
        assert_eq!(
            file.raw_file_json["js"]["menu"],
            serde_json::Value::String("dist/menu.runtime.js".to_string())
        );
    }

    #[test]
    fn parse_rejects_legacy_config_table() {
        let error = AppChordsFile::parse(
            r#"
[config]
name = "Legacy"

[chords]
a = { name = "Alpha" }
"#,
        )
        .expect_err("legacy config table should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn parse_rejects_legacy_args_properties() {
        let error = AppChordsFile::parse(
            r#"
[chords]
a = { name = "Alpha", args = ["legacy"] }
"#,
        )
        .expect_err("legacy args property should be rejected");

        assert!(error.to_string().contains("use `js:<name>`"));
    }

    #[test]
    fn parse_rejects_raw_js_property() {
        let error = AppChordsFile::parse(
            r#"
[chords]
a = { name = "Alpha", js = ["legacy"] }
"#,
        )
        .expect_err("raw js property should be rejected");

        assert!(error.to_string().contains("`js` is not supported"));
    }

    #[test]
    fn parse_separates_regex_chords_from_exact_chords() {
        let file = AppChordsFile::parse(
            r#"
[chords]
a = { name = "Exact" }
'-(\d+)' = { name = "Regex $1", 'js:default' = "Number($1)" }
"#,
        )
        .expect("parse should succeed");

        assert!(file.chords.contains_key("a"));
        assert_eq!(file.regex_chords.len(), 1);
        assert_eq!(file.regex_chords[0].sequence_template, r#"-(\d+)"#);
    }

    #[test]
    fn build_chord_substitutes_capture_values() {
        let mut extra = HashMap::new();
        extra.insert("js:default".to_string(), toml::Value::String("Number($1)".to_string()));
        let value = AppChordMapValue::Single(AppChord {
            index: 7,
            name: "Menu Bar: Item $1".to_string(),
            shortcut: None,
            shell: None,
            extra,
        });
        let keys = Key::parse_sequence("-42").expect("valid sequence");
        let chord = value
            .build_chord_for_keys(keys.clone(), &["42".to_string()], r#"-(\d+)"#)
            .expect("chord should build");

        assert_eq!(chord.keys, keys);
        assert_eq!(chord.name, "Menu Bar: Item 42");
        assert_eq!(
            chord.js,
            Some(ChordJsInvocation {
                export_name: "default".to_string(),
                args: ChordJsArgs::Eval("Number(42)".to_string()),
            })
        );
    }
}
