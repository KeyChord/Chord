use super::AppFrontmost;
use crate::app::AppSingleton;
use crate::state::{FrontmostObservable, FrontmostState, Observable};
use anyhow::Result;
use nject::provider;
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
