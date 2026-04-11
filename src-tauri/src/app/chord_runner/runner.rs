use super::{
    HandlerChordActionTaskRun, HandlerChordActionTaskRunner, ShellChordActionTaskRun,
    ShellChordActionTaskRunner, ShortcutChordActionTaskRun, ShortcutChordActionTaskRunner,
};
use crate::models::{ChordInputEvent, ChordTaskAction, FilePathslug};
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

#[derive(Debug)]
pub enum ChordActionTaskRun {
    Shell(ShellChordActionTaskRun),
    Shortcut(ShortcutChordActionTaskRun),
    Handler(HandlerChordActionTaskRun),
}

#[injectable]
pub struct ChordActionTaskRunner {
    handler: HandlerChordActionTaskRunner,
    shell: ShellChordActionTaskRunner,
    pub shortcut: ShortcutChordActionTaskRunner,
}

impl ChordActionTaskRunner {
    /// Called when the chord keys are pressed down.
    pub fn start_task(&self, task: &ChordActionTask) -> anyhow::Result<ChordActionTaskRun> {
        log::debug!("Starting task: {:?}", task);
        let task_run = match &task.action {
            ChordTaskAction::Handler(action) => {
                ChordActionTaskRun::Handler(self.handler.start(task, action)?)
            }
            ChordTaskAction::Shell(action) => {
                ChordActionTaskRun::Shell(self.shell.start(task, action)?)
            }
            ChordTaskAction::Shortcut(action) => {
                ChordActionTaskRun::Shortcut(self.shortcut.start(task, action)?)
            }
        };
        Ok(task_run)
    }

    /// Called when the chord keys are lifted. Async is needed for buffering chords.
    pub async fn end_task(&self, task_run: ChordActionTaskRun) -> anyhow::Result<()> {
        match task_run {
            ChordActionTaskRun::Handler(task_run) => self.handler.end(task_run).await?,
            ChordActionTaskRun::Shell(task_run) => self.shell.end(task_run).await?,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.end(task_run).await?,
        };
        Ok(())
    }

    /// Called if the user explicitly presses `Esc` or reloads the config
    #[allow(dead_code)]
    pub fn abort_task(&self, task_run: ChordActionTaskRun) -> anyhow::Result<()> {
        match task_run {
            ChordActionTaskRun::Handler(task_run) => self.handler.abort(task_run)?,
            ChordActionTaskRun::Shell(task_run) => self.shell.abort(task_run)?,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.abort(task_run)?,
        };
        Ok(())
    }
}
