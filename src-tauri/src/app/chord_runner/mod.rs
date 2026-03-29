use crate::app::SafeAppHandle;
use anyhow::Result;
use std::sync::Arc;
use serde::Serialize;
use typeshare::typeshare;
use crate::app::chord_runner::shortcut::ChordShortcutActionRunner;
use crate::models::{Chord, ChordAction, ChordJavascriptAction, ChordShellAction, ChordShortcutAction};
use async_trait::async_trait;
use crate::app::chord_runner::javascript::{ChordActionTaskJavascriptRunner, JavascriptChordActionTaskRunner};
use crate::app::chord_runner::shell::ShellChordActionTaskRunner;

pub mod javascript;
pub mod registry;
pub mod runtime;
pub mod shell;
pub mod shortcut;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ChordActionTask {
    pub action: ChordAction,
    pub num_times: u32
}

pub trait ChordActionTaskRun {
    fn id(&self) -> u32;
}

trait ChordActionTaskRunner {
    /// Called when the chord keys are pressed down.
    fn start(&self, task: ChordActionTask) -> Result<Option<Box<dyn ChordActionTaskRun>>>;

    /// Called when the chord keys are lifted. Can be a no-op.
    fn end(&self, task_run: dyn ChordActionTaskRun) -> Result<Option<()>>;

    /// Called if the user explicitly presses `Esc` or reloads the config
    fn abort(&self, task_run: dyn ChordActionTaskRun) -> Result<Option<Box<dyn ChordActionTaskRun>>>;
}

struct ChordActionTaskRunnerRegistry {
    runners: Vec<Arc<dyn ChordActionTaskRunner>>
}

impl ChordActionTaskRunnerRegistry {
    pub fn with_default_runners(handle: SafeAppHandle) -> Self {
        Self {
            runners: vec![
                Arc::new(JavascriptChordActionTaskRunner::new(handle)),
                Arc::new(ShellChordActionTaskRunner::new(handle)),
                Arc::new(ChordShortcutActionRunner::new(handle)),
            ],
        }
    }
}
