use std::collections::HashMap;
use serde::Serialize;
use typeshare::typeshare;
use crate::app::chord_package_manager::ChordJsPackage;
use crate::app::chord_runner::ChordActionTask;
use crate::models::{Chord, ChordAction, ChordInput, ChordString, ChordTaskAction, ChordsFile, HandlerChordAction};
use anyhow::Result;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackage {
    /// The `name` property of the `package.json` file; defaults to the folder name if not present.
    pub name: String,

    pub js_package: Option<ChordJsPackage>,

    pub app_chords_files: HashMap<String, ChordsFile>,
    pub global_chords: HashMap<ChordString, ChordReference>
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordReference {
    pub chords_file_path: String,
    pub chord: Chord,
}

impl ChordPackage {
    pub fn resolve_chord_for_input(&self, input: &ChordInput) -> Option<ChordReference> {
        if let Some(app_id) = &input.application_id {
            let chords_file_path = format!("{}/macos.toml", app_id.replace(".", "/"));
            if let Some(chords_file) = self.app_chords_files.get(app_id) {
                if let Some(chord) = chords_file.chords.iter().find(|c| c.trigger.matches(&input.keys)) {
                    return Some(ChordReference { chord: chord.clone(), chords_file_path });
                }
            }
        }

        self.global_chords.values().find(|c| c.chord.trigger.matches(&input.keys))
            .map(|c| ChordReference {
                chords_file_path: c.chords_file_path.clone(),
                chord: c.chord.clone()
            })
    }

    pub fn resolve_task(&self, chord_reference: ChordReference, action: ChordAction, num_times: u32) -> Result<Option<ChordActionTask>> {
        Ok(match action {
            ChordAction::Emit(emit) => {
                let Some(chords_file) = self.app_chords_files.get(&chord_reference.chords_file_path) else {
                    return Ok(None);
                };

                // For now, we assume the handler for the event that was emitted lives in the same file
                let handler = chords_file.handlers.get(&emit.event_key);
                if let Some(handler) = handler {
                    Some(ChordActionTask { action: ChordTaskAction::Handler(HandlerChordAction {
                        file: handler.file.clone(),
                        handler_args: handler.args.clone(),
                        event_args: emit.args.clone()
                    }), num_times })
                } else {
                    None
                }
            },
            ChordAction::Shortcut(shortcut) => {
                Some(ChordActionTask { action: ChordTaskAction::Shortcut(shortcut), num_times })
            }
            ChordAction::Shell(shell) => {
                Some(ChordActionTask { action: ChordTaskAction::Shell(shell), num_times })
            }
        })
    }
}