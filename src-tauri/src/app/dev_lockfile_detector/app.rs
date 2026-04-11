use super::DevLockfileDetector;
use crate::app::AppSingleton;
use crate::state::DesktopAppManagerObservable;
use anyhow::Result;
use nject::provider;
use std::path::PathBuf;
use tauri::AppHandle;

#[provider]
pub struct DevLockfileDetectorProvider;

impl AppSingleton for DevLockfileDetector {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
