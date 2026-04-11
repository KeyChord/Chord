use crate::app::chord_runner::{
    ChordActionTaskRunner, HandlerChordActionTaskRunner, ShellChordActionTaskRunner,
    ShortcutChordActionTaskRunner,
};
use crate::app::state::AppSingleton;
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct ChordActionTaskRunnerProvider {
    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for ChordActionTaskRunner {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
