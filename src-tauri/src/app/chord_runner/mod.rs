use anyhow::Result;
use serde::Serialize;
use typeshare::typeshare;
use crate::models::ChordAction;
use self::shortcut::{ShortcutChordActionTaskRun, ShortcutChordActionTaskRunner};
use self::javascript::{JavascriptChordActionTaskRun, JavascriptChordActionTaskRunner};
use self::shell::{ShellChordActionTaskRun, ShellChordActionTaskRunner};

pub mod javascript;
pub mod shell;
pub mod shortcut;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ChordActionTask {
    pub action: ChordAction,
    pub num_times: u32
}

pub enum ChordActionTaskRun {
    Javascript(JavascriptChordActionTaskRun),
    Shell(ShellChordActionTaskRun),
    Shortcut(ShortcutChordActionTaskRun)
}

struct ChordActionTaskRunner {
    javascript: JavascriptChordActionTaskRunner,
    shell: ShellChordActionTaskRunner,
    shortcut: ShortcutChordActionTaskRunner
}

impl ChordActionTaskRunner {
    /// Called when the chord keys are pressed down.
    pub fn start(&self, task: ChordActionTask) -> Result<ChordActionTaskRun> {
        let task_run = match task.action {
            ChordAction::Javascript(action) => ChordActionTaskRun::Javascript(self.javascript.start(action, task.num_times)?),
            ChordAction::Shell(action) => ChordActionTaskRun::Shell(self.shell.start(action, task.num_times)?),
            ChordAction::Shortcut(action) => ChordActionTaskRun::Shortcut(self.shortcut.start(action, task.num_times)?)
        };
        Ok(task_run)
    }

    /// Called when the chord keys are lifted. Async is needed for buffering chords.
    pub async fn end(&self, task_run: ChordActionTaskRun) -> Result<()> {
        match task_run {
            ChordActionTaskRun::Javascript(task_run) => self.javascript.end(task_run).await?,
            ChordActionTaskRun::Shell(task_run) => self.shell.end(task_run).await?,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.end(task_run).await?
        };
        Ok(())
    }

    /// Called if the user explicitly presses `Esc` or reloads the config
    pub fn abort(&self, task_run: ChordActionTaskRun) -> Result<()> {
        match task_run {
            ChordActionTaskRun::Javascript(task_run) => self.javascript.abort(task_run)?,
            ChordActionTaskRun::Shell(task_run) => self.shell.abort(task_run)?,
            ChordActionTaskRun::Shortcut(task_run) => self.shortcut.abort(task_run)?
        };
        Ok(())
    }
}
