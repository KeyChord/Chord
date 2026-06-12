use crate::app::chord_mode_manager::{ChordInputManager, ChordModePanel, NativeSurfaceRect};
use crate::state::{
    ChordInputObservable, ChordInputState, ChordPanelObservable, ChordPanelState, Observable,
};
use anyhow::Result;
use nject::injectable;
use parking_lot::Mutex;
use std::collections::HashSet;
use keycode::KeyMappingCode;
use tauri::{AppHandle, Listener};
use crate::models::{Key, KeyEvent};

#[injectable]
pub struct ChordModeManager {
    pub panel: ChordModePanel,
    input_manager: ChordInputManager,

    observable: ChordPanelObservable,
    handle: AppHandle,
}

impl ChordModeManager {
    pub(super) fn init(&self) -> Result<()> {
        self.panel.init();

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

    pub fn ensure_active(&self) -> Result<()> {
        log::debug!("Activating Chord Mode");
        if self.panel.ensure_visible()? {
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
