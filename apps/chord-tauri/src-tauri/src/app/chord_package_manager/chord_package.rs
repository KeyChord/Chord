use crate::app::chord_package_manager::ChordJsPackage;
use crate::app::chord_runner::ChordActionTask;
use crate::models::{
    Chord, ChordAction, ChordEventTrigger, ChordInput, ChordInputEvent, ChordKeyboardEvent,
    ChordTaskAction, ChordTrigger, CompiledChordsFile, FilePathslug, HandlerChordAction, Key,
    RawChordsFile, is_core_chords_file,
};
use anyhow::{Context, Result};
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

/// The app a chords file applies to: `chords/com/spotify/client/macos.toml` applies to
/// `com.spotify.client`. Root and bundled (`chords/@scope/…`) files apply to no app.
pub fn chords_file_app_id(pathslug: &Path) -> Option<String> {
    if pathslug.file_name()? != "macos.toml" {
        return None;
    }
    let app_id = pathslug
        .parent()?
        .strip_prefix("chords")
        .ok()?
        .to_str()?
        .replace('/', ".");
    (!app_id.is_empty() && !app_id.starts_with('@')).then_some(app_id)
}

impl ChordPackage {
    pub fn matching_app_chords(&self, event: &ChordInputEvent) -> Vec<ChordReference> {
        let Some(app_id) = &event.application_id else {
            return Vec::new();
        };
        self.matching_app_chords_for_id(event, app_id)
    }

    /// Match an additional UI context without changing the real focused app
    /// carried by the event and exposed to JavaScript handlers.
    pub fn matching_app_chords_for_id(
        &self,
        event: &ChordInputEvent,
        app_id: &str,
    ) -> Vec<ChordReference> {
        let pathslug = PathBuf::from(format!("chords/{}/macos.toml", app_id.replace(".", "/")));
        let Some(file) = self.compiled_chords_files.get(&pathslug) else {
            return Vec::new();
        };
        file.chords
            .iter()
            .filter(|chord| chord.trigger.matches_in_namespace(&event.input, is_core_chords_file(&self.name, &pathslug)))
            .map(|chord| ChordReference {
                package_name: self.name.clone(),
                chords_file_pathslug: pathslug.clone(),
                chord: chord.clone(),
            })
            .collect()
    }

    pub fn matching_global_chords(&self, event: &ChordInputEvent) -> Vec<ChordReference> {
        self.global_chords
            .iter()
            .filter(|reference| reference.chord.trigger.matches_in_namespace(&event.input,
                is_core_chords_file(&reference.package_name, &reference.chords_file_pathslug)))
            .cloned()
            .collect()
    }

    pub fn resolve_chord_from_input_event(
        &self,
        event: &ChordInputEvent,
    ) -> Option<ChordReference> {
        self.matching_app_chords(event)
            .into_iter()
            .chain(self.matching_global_chords(event))
            .find(|reference| {
                reference
                    .chord
                    .actions
                    .iter()
                    .any(|action| action.trigger() == ChordEventTrigger::Input)
            })
    }

    pub fn resolve_task(
        &self,
        event: ChordInputEvent,
        chord_reference: ChordReference,
        num_times: u32,
    ) -> Result<Option<ChordActionTask>> {
        let Some(action) = chord_reference
            .chord
            .actions
            .iter()
            .find(|action| action.trigger() == ChordEventTrigger::Input)
        else {
            return Ok(None);
        };
        self.resolve_action_task(event, &chord_reference, action, num_times, None, None)
    }

    pub fn resolve_action_task(
        &self,
        event: ChordInputEvent,
        chord_reference: &ChordReference,
        action: &ChordAction,
        num_times: u32,
        sequence_id: Option<String>,
        keyboard_event: Option<ChordKeyboardEvent>,
    ) -> Result<Option<ChordActionTask>> {
        Ok(match action {
            // Core actions execute synchronously in the input manager so a halt
            // never queues behind the task it must stop.
            ChordAction::Builtin(_) => None,
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
                let event_args = match &chord_reference.chord.trigger {
                    ChordTrigger::Pattern(regex) => {
                        // Replace all $1, $2, ... with the captured match from the regex
                        let mut args = Vec::new();
                        let input_string = Key::serialize_sequence(&event.input)
                            .context("invalid chord sequence")?;

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
                    let input = Key::serialize_sequence(&event.input).unwrap_or_default();
                    Some(ChordActionTask {
                        package_name: chord_reference.package_name.clone(),
                        event,
                        initiator_file_pathslug: chord_reference.chords_file_pathslug.clone(),
                        action: ChordTaskAction::Handler(HandlerChordAction {
                            handler_id: handler.handler_id.clone(),
                            event_args,
                            input,
                            sequence_id,
                            assign_to: emit.assign_to.clone(),
                            keyboard_event,
                            // Captured when the run starts, after dispatch.
                            focused_window_id: None,
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
                package_name: chord_reference.package_name.clone(),
                event,
                initiator_file_pathslug: chord_reference.chords_file_pathslug.clone(),
                action: ChordTaskAction::Shortcut(shortcut.clone()),
                num_times,
            }),
            ChordAction::Shell(shell) => Some(ChordActionTask {
                package_name: chord_reference.package_name.clone(),
                event,
                initiator_file_pathslug: chord_reference.chords_file_pathslug.clone(),
                action: ChordTaskAction::Shell(shell.clone()),
                num_times,
            }),
        })
    }
}
