use crate::app::chord_package_manager::ChordPackageManager;
use crate::state::{
    ChordPackageManagerObservable, FrontmostObservable, FrontmostState, HintPanelObservable,
    HintPanelState, Observable,
};
use anyhow::Result;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Keeps `HintPanelState` to what the panel draws for the frontmost app, recomputing it
/// when that app or the packages change.
pub fn feed_hint_panel(
    handle: &AppHandle,
    hint_panel: HintPanelObservable,
    frontmost: &FrontmostObservable,
    packages: &ChordPackageManagerObservable,
) -> Result<()> {
    let refresh = {
        let handle = handle.clone();
        Arc::new(move |frontmost: &FrontmostState| {
            // Packages load after setup; their first load refreshes this again.
            let Some(manager) = handle.try_state::<ChordPackageManager>() else {
                return;
            };
            let packages = manager.hint_panel_packages(
                frontmost.chord_application(),
                frontmost.chord_file_picker_active(),
            );
            if let Err(error) = hint_panel.set_state(|_| HintPanelState { packages }) {
                log::error!("Failed to update the Hint Panel's chords: {error:#}");
            }
        })
    };
    frontmost.subscribe({
        let refresh = refresh.clone();
        Arc::new(move |_, frontmost| refresh(frontmost))
    })?;
    let frontmost = frontmost.provide();
    packages.subscribe(Arc::new(move |_, _| match frontmost.get_state() {
        Ok(state) => refresh(&state),
        Err(error) => log::error!("Failed to read the frontmost app: {error:#}"),
    }))?;
    Ok(())
}
