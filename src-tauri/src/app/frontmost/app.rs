use super::AppFrontmost;
use crate::app::AppSingleton;
use crate::app::frontmost::frontmost::FrontmostTracker;
use crate::state::{FrontmostObservable, FrontmostState, Observable};
use anyhow::Result;
use frontmost::{Detector, start_nsrunloop};
use nject::provider;
use objc2_app_kit::NSWorkspace;
use observable_property::ObservableProperty;
use std::sync::Arc;
use std::thread;
use tauri::AppHandle;

#[provider]
pub struct AppFrontmostProvider {
    #[provide(FrontmostObservable, |v| v.provide())]
    pub frontmost_observable: FrontmostObservable,
    
    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for AppFrontmost {
    fn init(&self) -> Result<()> {
        self.init()
    }
}
