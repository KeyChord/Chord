use super::DevLockfileDetector;
use super::lease::{InputLease, is_development};
use crate::app::{AppHandleExt, AppSingleton};
use anyhow::Result;
use nject::provider;
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[provider]
pub struct DevLockfileDetectorProvider {
    #[provide(AppHandle, |h| h.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for DevLockfileDetector {
    fn init(&self) -> Result<()> {
        // local_data_dir is user-scoped, not bundle-scoped: every channel must
        // open the same inodes, even with a different TMPDIR or bundle suffix.
        let directory = self
            .handle
            .path()
            .local_data_dir()?
            .join("com.leonsilicon.chord/input-ownership-v1");
        let mut lease =
            InputLease::open(&directory, is_development(&self.handle.config().identifier))?;
        let handle = self.handle.clone();
        std::thread::Builder::new()
            .name("chord-input-owner".into())
            .spawn(move || {
                let mut next_generation = 0;
                let mut changed = |active| {
                    let detector = handle.app_state().dev_lockfile_detector();
                    let next = if active {
                        next_generation += 1;
                        Some(next_generation)
                    } else {
                        None
                    };
                    detector.gate.transition(next, || {
                        handle.app_state().keyboard().reset_input_state();
                        if let Err(error) = handle.app_state().app_controller().set_idle_mode() {
                            log::error!(
                                "Failed to clear input state during ownership handoff: {error:#}"
                            );
                        }
                    });
                    log::info!(
                        "Input ownership: {} ({})",
                        if active { "active" } else { "standby" },
                        handle.config().identifier
                    );
                };
                loop {
                    let eligible = handle.app_state().permissions().can_intercept_input()
                        && handle.app_state().keyboard().input_handlers_running();
                    if let Err(error) = lease.refresh(eligible, &mut changed) {
                        // Fail closed. Drop releases both leases only after the gate
                        // has been closed; another process can safely take over.
                        changed(false);
                        log::error!("Input ownership failed; interception disabled: {error:#}");
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
            })?;
        Ok(())
    }
}
