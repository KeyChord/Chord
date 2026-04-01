use crate::app::chord_package_manager::ChordJsPackage;
use crate::app::chord_runner::ChordActionTask;
use crate::models::{
    Chord, ChordAction, ChordInput, ChordTaskAction, ChordTrigger, CompiledChordsFile,
    FilePathslug, HandlerChordAction, RawChordsFile,
};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackage {
    /// The `name` property of the `package.json` file; defaults to the folder name if not present.
    pub name: String,

    pub js_package: Option<ChordJsPackage>,

    pub raw_chords_files: HashMap<FilePathslug, RawChordsFile>,
    pub compiled_chords_files: HashMap<FilePathslug, CompiledChordsFile>,
    pub global_chords: Vec<ChordReference>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordReference {
    pub package_name: String,
    pub chords_file_pathslug: FilePathslug,
    pub chord: Chord,
}

impl ChordPackage {
    pub fn resolve_chord_for_input(&self, input: &ChordInput) -> Option<ChordReference> {
        if let Some(app_id) = &input.application_id {
            let pathslug = format!("chords/{}/macos.toml", app_id.replace(".", "/"));
            let chords_file_pathslug = Path::new(&pathslug);
            if let Some(chords_file) = self.compiled_chords_files.get(chords_file_pathslug) {
                if let Some(chord) = chords_file
                    .chords
                    .iter()
                    .find(|c| c.trigger.matches(&input.keys))
                {
                    return Some(ChordReference {
                        package_name: self.name.clone(),
                        chord: chord.clone(),
                        chords_file_pathslug: chords_file_pathslug.to_path_buf(),
                    });
                }
            }
        }

        self.global_chords
            .iter()
            .find(|c| c.chord.trigger.matches(&input.keys))
            .map(|c| ChordReference {
                package_name: self.name.clone(),
                chords_file_pathslug: c.chords_file_pathslug.clone(),
                chord: c.chord.clone(),
            })
    }

    pub fn resolve_task(
        &self,
        input: &ChordInput,
        chord_reference: ChordReference,
        num_times: u32,
    ) -> Result<Option<ChordActionTask>> {
        let Some(action) = chord_reference.chord.actions.first() else {
            return Ok(None);
        };

        Ok(match action {
            ChordAction::Emit(emit) => {
                let Some(chords_file) = self
                    .compiled_chords_files
                    .get(&chord_reference.chords_file_pathslug)
                else {
                    anyhow::bail!(
                        "referenced chords file not found: {:?}",
                        chord_reference.chords_file_pathslug
                    );
                };

                let scoped_event_key =
                    format!("{}:{}", chord_reference.package_name, emit.event_key);
                let scoped_event_handler = chords_file
                    .handlers
                    .iter()
                    .find(|handler| handler.event == scoped_event_key);
                let handler = scoped_event_handler.or_else(|| {
                    chords_file
                        .handlers
                        .iter()
                        .find(|handler| handler.event == emit.event_key)
                });
                let event_args = match chord_reference.chord.trigger {
                    ChordTrigger::Pattern(regex) => {
                        // Replace all $1, $2, ... with the captured match from the regex
                        let mut args = Vec::new();
                        let input_string = input
                            .keys
                            .iter()
                            .map(|k| k.to_char(false).unwrap_or_default())
                            .collect::<String>();

                        log::debug!(
                            "replacing capture args with regex {:?} matching {}",
                            regex,
                            input_string
                        );

                        for arg in &emit.args {
                            if let Some(arg) = arg.as_str() {
                                if arg.starts_with("$") {
                                    if let Ok(index) = arg[1..].parse::<usize>() {
                                        if let Some(captures) = regex.captures(&input_string) {
                                            args.push(toml::Value::String(
                                                captures
                                                    .get(index)
                                                    .map_or("", |m| m.as_str())
                                                    .to_string(),
                                            ));
                                            continue;
                                        }
                                    }
                                }
                            }

                            args.push(arg.clone());
                        }
                        args
                    }
                    ChordTrigger::Keys(_keys) => emit.args.clone(),
                };
                if let Some(handler) = handler {
                    Some(ChordActionTask {
                        package_name: chord_reference.package_name,
                        initiator_file_pathslug: chord_reference.chords_file_pathslug,
                        action: ChordTaskAction::Handler(HandlerChordAction {
                            file: handler.file.clone(),
                            build_args: handler.args.clone(),
                            event_args,
                        }),
                        num_times,
                    })
                } else {
                    log::debug!(
                        "missing handler for event: {}, available handlers: {:?}",
                        emit.event_key,
                        chords_file
                            .handlers
                            .iter()
                            .map(|h| &h.event)
                            .collect::<Vec<_>>()
                    );
                    None
                }
            }
            ChordAction::Shortcut(shortcut) => Some(ChordActionTask {
                package_name: chord_reference.package_name,
                initiator_file_pathslug: chord_reference.chords_file_pathslug,
                action: ChordTaskAction::Shortcut(shortcut.clone()),
                num_times,
            }),
            ChordAction::Shell(shell) => Some(ChordActionTask {
                package_name: chord_reference.package_name,
                initiator_file_pathslug: chord_reference.chords_file_pathslug,
                action: ChordTaskAction::Shell(shell.clone()),
                num_times,
            }),
        })
    }
}
