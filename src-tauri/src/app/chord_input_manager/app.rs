use std::collections::HashSet;
use parking_lot::Mutex;
use tauri::{AppHandle, Listener};
use crate::app::AppSingleton;
use crate::app::chord_input_manager::AppChordInputManager;
use crate::state::{ChordInputObservable, Observable};

impl AppSingleton<ChordInputObservable> for AppChordInputManager {
    fn new(handle: AppHandle) -> Self {
        Self {
            active_task_run: Mutex::new(None),
            held_keys: Mutex::new(HashSet::new()),
            handle,
            observable: ChordInputObservable::uninitialized()
        }
    }


    fn init(&self, observable: ChordInputObservable) -> anyhow::Result<()> {
        let surface_window = self.ui.get_or_create_window()?;
        let surface_handle = self.ui.handle.clone();
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

}

