use crate::app::AppSingleton;
use crate::app::chord_mode_manager::ChordModeManager;
use crate::state::{ChordInputObservable, ChordModeObservable, FrontmostObservable, Observable};
use anyhow::Result;
use nject::provider;
use parking_lot::Mutex;
use std::collections::HashSet;
use tauri::{AppHandle, Listener};

#[provider]
pub struct ChordModeManagerProvider {
    pub chord_mode_observable: ChordModeObservable,
    pub chord_input_observable: ChordInputObservable,
    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for ChordModeManager {
    fn init(&self) -> Result<()> {
        self.init()
    }
}
