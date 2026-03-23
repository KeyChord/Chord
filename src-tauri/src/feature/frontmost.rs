use std::sync::Arc;
use std::thread;
use frontmost::{start_nsrunloop, Detector};
use objc2_app_kit::NSWorkspace;
use arc_swap::ArcSwap;

#[derive(Debug)]
struct FrontmostTracker {
    pub frontmost_application_id: Arc<ArcSwap<Option<String>>>,
}

#[cfg(target_os = "macos")]
impl frontmost::app::FrontmostApp for FrontmostTracker {
    fn set_frontmost(&mut self, new_value: &str) {
        self.frontmost_application_id.store(Arc::new(Some(new_value.to_string())));
    }

    fn update(&mut self) {
        println!("Application activated: {:?}", self.frontmost_application_id);
    }
}

pub struct AppFrontmost {
    pub frontmost_application_id: Arc<ArcSwap<Option<String>>>,
}

impl AppFrontmost {
   pub fn new_with_detector() -> Self {
       let mut initial_bundle_id: Option<String> = None;
       let workspace = NSWorkspace::sharedWorkspace();
       if let Some(application) = workspace.frontmostApplication() {
           if let Some(bundle_id) = application.bundleIdentifier() {
               initial_bundle_id = Some(bundle_id.to_string())
           }
       }

       let shared = Arc::new(ArcSwap::new(Arc::new(initial_bundle_id)));
       let frontmost_tracker = FrontmostTracker {
           frontmost_application_id: shared.clone()
       };
       thread::spawn(|| {
           start_nsrunloop!();
       });
       Detector::init(Box::new(frontmost_tracker));

       Self {
            frontmost_application_id: shared
       }
   }
}

