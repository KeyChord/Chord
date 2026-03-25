use crate::observables::{FrontmostObservable, FrontmostState, Observable};
use anyhow::Result;
use frontmost::{Detector, start_nsrunloop};
use objc2_app_kit::NSWorkspace;
use std::sync::Arc;
use std::thread;

#[derive(Debug)]
struct FrontmostTracker {
    pub observable: Arc<FrontmostObservable>,
}

#[cfg(target_os = "macos")]
impl frontmost::app::FrontmostApp for FrontmostTracker {
    fn set_frontmost(&mut self, new_value: &str) {
        let _ = self.observable.set_state(FrontmostState {
            frontmost_app_bundle_id: Some(new_value.to_string()),
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
    observable: Arc<FrontmostObservable>,
}

impl AppFrontmost {
    pub fn new_with_detector(observable: Arc<FrontmostObservable>) -> Result<Self> {
        let workspace = NSWorkspace::sharedWorkspace();
        if let Some(application) = workspace.frontmostApplication() {
            if let Some(bundle_id) = application.bundleIdentifier() {
                observable.set_state(FrontmostState {
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

        Ok(Self { observable })
    }
}
