use crate::app::state::AppSingleton;
use anyhow::Result;
use frontmost::{Detector, start_nsrunloop};
use objc2_app_kit::NSWorkspace;
use std::thread;
use tauri::AppHandle;
use crate::state::{FrontmostObservable, FrontmostState, Observable};

#[derive(Debug)]
pub(super) struct FrontmostTracker {
    pub(super) observable: FrontmostObservable,
}

#[cfg(target_os = "macos")]
impl frontmost::app::FrontmostApp for FrontmostTracker {
    fn set_frontmost(&mut self, new_value: Option<String>) {
        let _ = self.observable.set_state(FrontmostState {
            frontmost_app_bundle_id: new_value,
        });
    }

    fn update(&mut self) {
        println!(
            "Application activated: {:?}",
            self.observable.get_state().unwrap().frontmost_app_bundle_id
        );
    }
}

pub struct AppFrontmost {
    pub(super) observable: FrontmostObservable,
    pub(super) handle: AppHandle,
}
