use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::Serialize;
use typeshare::typeshare;
use crate::app::chord_package_manager::ChordJsPackage;
use crate::app::chord_runner::ChordActionTask;
use crate::models::{Chord, ChordAction, ChordInput, ChordTaskAction, ChordsFile, HandlerChordAction};
use anyhow::Result;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackage {
    /// The `name` property of the `package.json` file; defaults to the folder name if not present.
    pub name: String,

    pub js_package: Option<ChordJsPackage>,

    pub app_chords_files: HashMap<PathBuf, ChordsFile>,
    pub global_chords: Vec<ChordReference>
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordReference {
    pub package_name: String,
    pub chords_file_path: PathBuf,
    pub chord: Chord,
}

impl ChordPackage {
    pub fn resolve_chord_for_input(&self, input: &ChordInput) -> Option<ChordReference> {
        if let Some(app_id) = &input.application_id {
            let path = format!("chords/{}/macos.toml", app_id.replace(".", "/"));
            let chords_file_path = Path::new(&path);
            if let Some(chords_file) = self.app_chords_files.get(chords_file_path) {
                if let Some(chord) = chords_file.chords.iter().find(|c| c.trigger.matches(&input.keys)) {
                    return Some(ChordReference {
                        package_name: self.name.clone(),
                        chord: chord.clone(),
                        chords_file_path: chords_file_path.to_path_buf()
                    });
                }
            }
        }

        self.global_chords.iter().find(|c| c.chord.trigger.matches(&input.keys))
            .map(|c| ChordReference {
                package_name: self.name.clone(),
                chords_file_path: c.chords_file_path.clone(),
                chord: c.chord.clone()
            })
    }

    pub fn resolve_task(&self, chord_reference: ChordReference, num_times: u32) -> Result<Option<ChordActionTask>> {
        let Some(action) = chord_reference.chord.actions.first() else {
            return Ok(None)
        };

        Ok(match action {
            ChordAction::Emit(emit) => {
                let Some(chords_file) = self.app_chords_files.get(&chord_reference.chords_file_path) else {
                    anyhow::bail!("referenced chords file not found: {:?}", chord_reference.chords_file_path);
                };

                // For now, we assume the handler for the event that was emitted lives in the same file
                let handler = chords_file.handlers.get(&emit.event_key);
                if let Some(handler) = handler {
                    Some(ChordActionTask {
                        package_name: chord_reference.package_name,
                        initiator_file_relpath: chord_reference.chords_file_path,
                        action: ChordTaskAction::Handler(HandlerChordAction {
                        file: handler.file.clone(),
                        build_args: handler.args.clone(),
                        event_args: emit.args.clone()
                    }), num_times })
                } else {
                    log::debug!("missing handler for event: {}, available handlers: {:?}", emit.event_key, chords_file.handlers.keys());
                    None
                }
            },
            ChordAction::Shortcut(shortcut) => {
                Some(ChordActionTask {
                    package_name: chord_reference.package_name,
                    initiator_file_relpath: chord_reference.chords_file_path,
                    action: ChordTaskAction::Shortcut(shortcut.clone()), num_times
                })
            }
            ChordAction::Shell(shell) => {
                Some(ChordActionTask {
                    package_name: chord_reference.package_name,
                    initiator_file_relpath: chord_reference.chords_file_path,
                    action: ChordTaskAction::Shell(shell.clone()), num_times
                })
            }
        })
    }
}