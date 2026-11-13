use crate::app::AppHandleExt;
use crate::app::chord_mode_manager::{ChordInputManager, ChordModePanel, NativeSurfaceRect};
use crate::models::{Key, KeyEvent};
use crate::state::{
    ChordInputObservable, ChordInputState, ChordPanelObservable, ChordPanelState, Observable,
};
use anyhow::Result;
use keycode::KeyMappingCode;
use nject::injectable;
use parking_lot::Mutex;
use std::collections::HashSet;
use tauri::{AppHandle, Listener};

#[injectable]
pub struct ChordModeManager {
    pub panel: ChordModePanel,
    input_manager: ChordInputManager,

    observable: ChordPanelObservable,
    handle: AppHandle,
}

impl ChordModeManager {
    pub(super) fn init(&self) -> Result<()> {
        self.input_manager.load_macros()?;
        self.panel.init()?;

        let surface_window = self.panel.get_or_create_window()?;
        let surface_handle = self.handle.clone();
        let listener_window = surface_window.clone();
        let listener_window2 = surface_window.clone();
        listener_window.listen(
            "chorder-surface-rect",
            move |event| match serde_json::from_str::<NativeSurfaceRect>(event.payload()) {
                Ok(rect) => {
                    if let Err(error) = ChordModePanel::configure_window_surface(
                        &listener_window2,
                        surface_handle.clone(),
                        rect,
                    ) {
                        log::error!("Failed to configure native chorder surface: {error}");
                    }
                }
                Err(error) => {
                    log::error!("Failed to parse chorder surface rect: {error}");
                }
            },
        );
        Ok(())
    }

    pub fn handle_key_event(&self, key_event: &KeyEvent) -> Result<()> {
        self.input_manager.handle_key_event(key_event)
    }

    pub fn is_inserting(&self) -> bool {
        self.input_manager.is_inserting()
    }

    pub fn handle_insert_key_event(&self, key_event: &KeyEvent) -> Result<()> {
        self.input_manager.handle_insert_key_event(key_event)
    }

    pub fn finish_insert(&self) -> Result<()> {
        self.input_manager.finish_insert()
    }

    pub fn execute_sequence(&self, sequence: &str) -> Result<()> {
        self.input_manager.execute_sequence(sequence)
    }

    pub fn update_macro(
        &self,
        target: crate::models::ChordMacroName,
        sequence: Option<String>,
        previous: Option<crate::models::ChordMacroName>,
    ) -> Result<()> {
        self.input_manager.update_macro(target, sequence, previous)
    }

    pub fn stop_loops(&self) -> Result<()> {
        self.input_manager.stop_loops()
    }

    pub fn configure_meta_commands(
        &self,
        hints: Vec<crate::state::MetaChordHint>,
        halt_loop_template: String,
    ) -> Result<()> {
        self.input_manager
            .configure_meta_commands(hints, halt_loop_template)
    }

    pub fn ensure_active(&self) -> Result<()> {
        log::debug!("Activating Chord Mode");
        let is_hint_panel_hidden = self
            .handle
            .app_state()
            .settings()
            .is_hint_panel_hidden_by_default();
        if self.panel.ensure_visible(!is_hint_panel_hidden)? {
            let _ = self.panel.prepare_surface_before_reveal();
            self.panel.reveal()?;
        }
        Ok(())
    }

    pub fn ensure_inactive(&self) -> Result<()> {
        self.panel.ensure_hidden()?;
        self.input_manager.reset();
        Ok(())
    }

    fn toggle_panel(&self) -> Result<()> {
        self.panel.toggle()?;
        Ok(())
    }
}
