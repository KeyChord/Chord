use serde::Serialize;
use tauri::AppHandle;
use typeshare::typeshare;
use crate::app::chord_runner::{JavascriptChordActionTaskRun, JavascriptChordActionTaskRunner};
use crate::app::chord_runner::{ShellChordActionTaskRun, ShellChordActionTaskRunner};
use crate::app::chord_runner::{ShortcutChordActionTaskRun, ShortcutChordActionTaskRunner};
use crate::models::ChordAction;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ChordActionTask {
    pub action: ChordAction,
    pub num_times: u32
}

#[derive(Debug)]
pub enum ChordActionTaskRun {
    Javascript(JavascriptChordActionTaskRun),
    Shell(ShellChordActionTaskRun),
    Shortcut(ShortcutChordActionTaskRun)
}

pub struct ChordActionTaskRunner {
    javascript: JavascriptChordActionTaskRunner,
    shell: ShellChordActionTaskRunner,
    pub shortcut: ShortcutChordActionTaskRunner
}

impl ChordActionTaskRunner {
    pub fn new(handle: AppHandle) -> Self {
        Self {
            javascript:  JavascriptChordActionTaskRunner::new(handle.clone()),
            shell: ShellChordActionTaskRunner::new(handle.clone()),
            shortcut: ShortcutChordActionTaskRunner::new(handle.clone())
        }
    }

    /// Called when the chord keys are pressed down.
    pub fn start_task(&self, task: &ChordActionTask) -> anyhow::Result<ChordActionTaskRun> {
        let task_run = match &task.action {
            ChordAction::Javascript(action) => ChordActionTaskRun::Javascript(self.javascript.start(action, task.num_times)?),
            ChordAction::Shell(action) => ChordActionTaskRun::Shell(self.shell.start(action, task.num_times)?),
            ChordAction::Shortcut(action) => ChordActionTaskRun::Shortcut(self.shortcut.start(action, task.num_times)?)
        };
        Ok(task_run)
    }

    /// Called when the chord keys are lifted. Async is needed for buffering chords.
    pub async fn end_task(&self, task_run: ChordActionTaskRun) -> anyhow::Result<()> {
        match task_run {
            ChordActionTaskRun::Javascript(task_run) => self.javascript.end(task_run).await?,
            ChordActionTaskRun::Shell(task_run) => self.shell.end(task_run).await?,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.end(task_run).await?
        };
        Ok(())
    }

    /// Called if the user explicitly presses `Esc` or reloads the config
    pub fn abort_task(&self, task_run: ChordActionTaskRun) -> anyhow::Result<()> {
        match task_run {
            ChordActionTaskRun::Javascript(task_run) => self.javascript.abort(task_run)?,
            ChordActionTaskRun::Shell(task_run) => self.shell.abort(task_run)?,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.abort(task_run)?
        };
        Ok(())
    }
}
