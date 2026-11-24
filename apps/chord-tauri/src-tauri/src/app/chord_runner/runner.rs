use super::{
    HandlerChordActionTaskRun, HandlerChordActionTaskRunner, ShellChordActionTaskRun,
    ShellChordActionTaskRunner, ShortcutChordActionTaskRun, ShortcutChordActionTaskRunner,
    TypeChordActionTaskRun, TypeChordActionTaskRunner,
};
use crate::models::{ChordInputEvent, ChordTaskAction, FilePathslug};
use crate::tauri_app::play_failure_sound;
use nject::injectable;
use serde::Serialize;
use std::path::PathBuf;
use tauri::AppHandle;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordActionTask {
    pub package_name: String,
    pub initiator_file_pathslug: FilePathslug,

    /// The event that triggered the task.
    pub event: ChordInputEvent,

    pub action: ChordTaskAction,
    pub num_times: u32,
}

impl ChordActionTask {
    /// What the task does, for the log. Typed text is left out: only its length is given.
    pub fn summary(&self) -> String {
        let action = match &self.action {
            ChordTaskAction::Shell(action) => format!("shell `{}`", action.command),
            ChordTaskAction::Handler(action) => {
                let mut summary =
                    format!("JS handler from {}", self.initiator_file_pathslug.display());
                if !action.event_args.is_empty() {
                    let args: Vec<_> = action
                        .event_args
                        .iter()
                        .map(|arg| arg.to_string())
                        .collect();
                    summary.push_str(&format!(" with [{}]", args.join(", ")));
                }
                summary
            }
            ChordTaskAction::Shortcut(action) => {
                let chords: Vec<_> = action
                    .simulated_shortcut
                    .chords
                    .iter()
                    .map(|chord| {
                        let keys: Vec<_> = chord.keys.iter().map(ToString::to_string).collect();
                        keys.join("+")
                    })
                    .collect();
                format!("shortcut {}", chords.join(" "))
            }
            ChordTaskAction::Type(action) => {
                format!("type {} characters", action.text.chars().count())
            }
        };
        let times = if self.num_times > 1 {
            format!(" ×{}", self.num_times)
        } else {
            String::new()
        };
        format!("{}: {action}{times}", self.package_name)
    }
}

#[derive(Debug)]
pub enum ChordActionTaskRun {
    Shell(ShellChordActionTaskRun),
    Shortcut(ShortcutChordActionTaskRun),
    Handler(HandlerChordActionTaskRun),
    Type(TypeChordActionTaskRun),
}

#[injectable]
pub struct ChordActionTaskRunner {
    handler: HandlerChordActionTaskRunner,
    handle: AppHandle,
    shell: ShellChordActionTaskRunner,
    pub shortcut: ShortcutChordActionTaskRunner,
    typing: TypeChordActionTaskRunner,
}

impl ChordActionTaskRunner {
    /// Called when the chord keys are pressed down.
    pub fn start_task(&self, task: &ChordActionTask) -> anyhow::Result<ChordActionTaskRun> {
        log::debug!("Starting task: {:?}", task);
        let result = match &task.action {
            ChordTaskAction::Handler(action) => self
                .handler
                .start(task, action)
                .map(ChordActionTaskRun::Handler),
            ChordTaskAction::Shell(action) => self
                .shell
                .start(task, action)
                .map(ChordActionTaskRun::Shell),
            ChordTaskAction::Shortcut(action) => self
                .shortcut
                .start(task, action)
                .map(ChordActionTaskRun::Shortcut),
            ChordTaskAction::Type(action) => self
                .typing
                .start(task, action)
                .map(ChordActionTaskRun::Type),
        };
        if result.is_err() {
            play_failure_sound(&self.handle);
        }
        result
    }

    /// Called when the chord keys are lifted. Async is needed for buffering chords.
    pub async fn end_task(&self, task_run: &mut ChordActionTaskRun) -> anyhow::Result<()> {
        let result = match task_run {
            ChordActionTaskRun::Handler(task_run) => self.handler.end(task_run).await,
            ChordActionTaskRun::Shell(task_run) => self.shell.end(task_run).await,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.end(task_run).await,
            ChordActionTaskRun::Type(task_run) => self.typing.end(task_run).await,
        };
        if result.is_err() {
            play_failure_sound(&self.handle);
        }
        result
    }

    /// Cancel an in-flight action, including releasing held simulated keys.
    pub fn abort_task(&self, task_run: ChordActionTaskRun) -> anyhow::Result<()> {
        match task_run {
            ChordActionTaskRun::Handler(task_run) => self.handler.abort(task_run)?,
            ChordActionTaskRun::Shell(task_run) => self.shell.abort(task_run)?,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.abort(task_run)?,
            ChordActionTaskRun::Type(task_run) => self.typing.abort(task_run)?,
        };
        Ok(())
    }
}
