use std::thread;
use frontmost::{start_nsrunloop, Detector};
use objc2_app_kit::NSWorkspace;
use tauri::AppHandle;
use crate::app::AppSingleton;
use super::{AppFrontmost, FrontmostTracker};
use crate::state::{FrontmostObservable, FrontmostState};

impl AppSingleton<FrontmostObservable> for AppFrontmost {
    fn new(handle: AppHandle) -> Self {
        Self {
            observable: FrontmostObservable::uninitialized(),
            handle,
        }
    }

    fn init(&self, observable: FrontmostObservable) -> anyhow::Result<()> {
        self.observable.init(observable.clone());

        let workspace = NSWorkspace::sharedWorkspace();
        if let Some(application) = workspace.frontmostApplication() {
            if let Some(bundle_id) = application.bundleIdentifier() {
                self.observable.set_state(FrontmostState {
                    frontmost_app_bundle_id: Some(bundle_id.to_string()),
                })?;
            }
        }

        thread::spawn(|| {
            start_nsrunloop!();
        });
        Detector::init(Box::new(FrontmostTracker {
            observable: observable.clone(),
        }));

        Ok(())
    }
}

