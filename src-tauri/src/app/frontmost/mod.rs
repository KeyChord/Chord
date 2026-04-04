use crate::app::state::StateSingleton;
use crate::observables::{FrontmostObservable, FrontmostState, Observable};
use anyhow::Result;
use frontmost::{Detector, start_nsrunloop};
use objc2_app_kit::NSWorkspace;
use std::thread;
use tauri::AppHandle;

#[derive(Debug)]
struct FrontmostTracker {
    observable: FrontmostObservable,
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
    observable: FrontmostObservable,
    _handle: AppHandle,
}

impl StateSingleton for AppFrontmost {
    fn new(handle: AppHandle) -> Self {
        Self {
            _handle: handle,
            observable: FrontmostObservable::placeholder(),
        }
    }
}

impl AppFrontmost {
    pub fn init(&self, observable: FrontmostObservable) -> Result<()> {
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
