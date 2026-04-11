use crate::app::chord_mode_manager::{ChordInputManager, ChordModeIndicatorUi, NativeSurfaceRect};
use crate::state::{
    ChordInputObservable, ChordInputState, ChordModeObservable, ChordModeState, Observable,
};
use anyhow::Result;
use nject::injectable;
use parking_lot::Mutex;
use std::collections::HashSet;
use tauri::{AppHandle, Listener};

#[injectable]
pub struct ChordModeManager {
    pub ui: ChordModeIndicatorUi,
    input_manager: ChordInputManager,

    observable: ChordModeObservable,
    handle: AppHandle,
}

impl ChordModeManager {
    pub(super) fn init(&self) -> Result<()> {
        self.ui.init();

        let surface_window = self.ui.get_or_create_window()?;
        let surface_handle = self.handle.clone();
        let listener_window = surface_window.clone();
        let listener_window2 = surface_window.clone();
        listener_window.listen(
            "chorder-surface-rect",
            move |event| match serde_json::from_str::<NativeSurfaceRect>(event.payload()) {
                Ok(rect) => {
                    if let Err(error) = ChordModeIndicatorUi::configure_window_surface(
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

    pub fn ensure_active(&self) -> Result<()> {
        if self.ui.ensure_visible()? {
            let _ = self.ui.prepare_surface_before_reveal();
            self.ui.reveal()?;
        }
        Ok(())
    }

    pub fn ensure_inactive(&self) -> Result<()> {
        self.ui.ensure_hidden()?;
        self.input_manager.reset();
        Ok(())
    }

    pub fn toggle_indicator_visibility(&self) -> Result<()> {
        self.observable.set_state(|prev| ChordModeState {
            is_indicator_visible: true,
            ..prev
        })?;
        Ok(())
    }
}
