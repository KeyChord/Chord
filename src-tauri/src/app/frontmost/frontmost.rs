use crate::app::AppHandleExt;
use crate::app::state::AppSingleton;
use crate::state::{FrontmostObservable, FrontmostState, Observable};
use anyhow::Result;
use frontmost::{Detector, start_nsrunloop};
use nject::injectable;
use objc2_app_kit::NSWorkspace;
use observable_property::ObservableProperty;
use std::sync::Arc;
use std::thread;
use tauri::AppHandle;

#[derive(Debug)]
pub(super) struct FrontmostTracker {
    pub(super) handle: AppHandle,
}

#[cfg(target_os = "macos")]
impl frontmost::app::FrontmostApp for FrontmostTracker {
    fn set_frontmost(&mut self, new_value: Option<String>) {
        let frontmost = self.handle.app_state().frontmost();
        frontmost.set_frontmost(new_value);
    }

    fn update(&mut self) {
        let frontmost = self.handle.app_state().frontmost();
        println!(
            "Application activated: {:?}",
            frontmost.frontmost().unwrap()
        );
    }
}

#[injectable]
#[derive(Debug)]
pub struct AppFrontmost {
    /// Needs to be shared with `FrontmostTracker
    observable: FrontmostObservable,
    handle: AppHandle,
}

impl AppFrontmost {
    pub(super) fn init(&self) -> Result<()> {
        let workspace = NSWorkspace::sharedWorkspace();
        if let Some(application) = workspace.frontmostApplication() {
            if let Some(bundle_id) = application.bundleIdentifier() {
                self.observable.set_state(|_| FrontmostState {
                    frontmost_app_bundle_id: Some(bundle_id.to_string()),
                })?;
            }
        }

        thread::spawn(|| {
            start_nsrunloop!();
        });

        Detector::init(Box::new(FrontmostTracker {
            handle: self.handle.clone(),
        }));

        Ok(())
    }

    pub fn frontmost(&self) -> Result<Option<String>> {
        Ok(self.observable.get_state()?.frontmost_app_bundle_id)
    }

    pub(super) fn set_frontmost(&self, app_id: Option<String>) -> Result<()> {
        self.observable.set_state(|_| FrontmostState {
            frontmost_app_bundle_id: app_id,
        })
    }
}
