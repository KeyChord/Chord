use crate::app::chord_package::{AppChordMapValue, AppChordRegex, AppChordsFile, Chord};
use crate::input::Key;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MatchingChordInfo {
    pub scope: String,
    pub scope_kind: &'static str,
    pub sequence: Vec<Key>,
    pub chord: Chord,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MatchingDescriptionInfo {
    pub scope: String,
    pub scope_kind: &'static str,
    pub sequence: Vec<Key>,
    pub description: String,
}

pub struct ChordRuntime {
    pub path: String,

    pub chords: HashMap<Vec<Key>, Chord>,
    pub regex_chords: Vec<RegexChord>,
    pub descriptions: HashMap<Vec<Key>, String>,
    pub js: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ChordPayload {
    pub chord: Chord,
    pub num_times: usize,
}

#[derive(Debug, Clone)]
pub struct RegexChord {
    pub sequence_template: String,
    pub value: AppChordMapValue,
    regex: Regex,
}

pub(crate) const GLOBAL_CHORD_RUNTIME_ID: &str = "__global__";

impl ChordRuntime {
    #[allow(dead_code)]
    pub fn from_chords(path: String, chords: HashMap<Vec<Key>, Chord>) -> Result<Self> {
        Ok(Self {
            path,
            chords,
            regex_chords: Vec::new(),
            descriptions: HashMap::new(),
            js: HashMap::new(),
        })
    }

    pub fn from_file_shallow(
        path: String,
        chord_file: AppChordsFile,
        placeholder_bindings: &HashMap<String, String>,
    ) -> Result<Self> {
        let js = chord_file.js.clone().unwrap_or_default();

        // We intentionally keep global chords because they execute in this runtime
        let chords = chord_file.get_chords_shallow(placeholder_bindings);
        let regex_chords = chord_file
            .get_regex_chords_shallow()
            .into_iter()
            .filter_map(|template| match RegexChord::new(template) {
                Ok(chord) => Some(chord),
                Err(error) => {
                    log::warn!("Skipping invalid regex chord: {}", error);
                    None
                }
            })
            .collect();
        let descriptions = chord_file.get_descriptions_shallow();

        Ok(Self {
            path,
            js,
            chords,
            regex_chords,
            descriptions,
        })
    }

    pub fn javascript_module_path(&self, export_name: &str) -> Option<&String> {
        self.js.get(export_name)
    }

    pub fn get_chord(&self, sequence: &[Key]) -> Option<ChordPayload> {
        let split_idx = sequence
            .iter()
            .position(|k| !k.is_digit())
            .unwrap_or(sequence.len());
        let (digit_keys, chord_keys) = sequence.split_at(split_idx);
        let num_times = if digit_keys.is_empty() {
            1
        } else {
            let digits: String = digit_keys.iter().filter_map(|k| k.to_char(false)).collect();
            let num_times = digits.parse::<usize>().unwrap_or(1);
            num_times
        };
        if let Some(chord) = self.chords.get(chord_keys) {
            return Some(ChordPayload {
                chord: chord.clone(),
                num_times,
            });
        }

        self.regex_chords.iter().find_map(|regex_chord| {
            regex_chord
                .match_sequence(chord_keys)
                .map(|chord| ChordPayload { chord, num_times })
        })
    }
}

impl RegexChord {
    fn new(template: AppChordRegex) -> Result<Self> {
        let regex = Regex::new(&format!("^{}$", template.sequence_template)).map_err(|error| {
            anyhow::anyhow!(
                "invalid regex sequence `{}`: {}",
                template.sequence_template,
                error
            )
        })?;

        Ok(Self {
            sequence_template: template.sequence_template,
            value: template.value,
            regex,
        })
    }

    fn match_sequence(&self, sequence: &[Key]) -> Option<Chord> {
        let sequence_text = Key::serialize_sequence(sequence)?;
        let captures = self.regex.captures(&sequence_text)?;
        let capture_values = captures
            .iter()
            .skip(1)
            .flatten()
            .map(|capture| capture.as_str().to_string())
            .collect::<Vec<_>>();

        self.value
            .build_chord_for_keys(sequence.to_vec(), &capture_values, &self.sequence_template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::chord_runner::javascript::{ChordJsArgs, ChordJsInvocation};

    #[test]
    fn regex_chord_matches_and_substitutes_captures() {
        let chord_file = AppChordsFile::parse(
            r#"
[chords]
'-(\d+)' = { name = "Menu Bar: Item $1", 'js:default' = "Number($1)" }
"#,
        )
        .expect("file should parse");
        let runtime = ChordRuntime::from_file_shallow(
            "chords/macos.toml".to_string(),
            chord_file,
            &HashMap::new(),
        )
        .expect("runtime should build");
        let sequence = Key::parse_sequence("-42").expect("valid sequence");
        let payload = runtime
            .get_chord(&sequence)
            .expect("regex chord should match");

        assert_eq!(payload.num_times, 1);
        assert_eq!(payload.chord.keys, sequence);
        assert_eq!(payload.chord.name, "Menu Bar: Item 42");
        assert_eq!(
            payload.chord.js,
            Some(ChordJsInvocation {
                export_name: "default".to_string(),
                args: ChordJsArgs::Eval("Number(42)".to_string()),
            })
        );
    }
}
