use std::path::PathBuf;
use tauri::AppHandle;
use crate::app::AppSingleton;
use crate::app::dev_lockfile_detector::DevLockfileDetector;

impl AppSingleton<()> for DevLockfileDetector {
    fn new(_: AppHandle) -> Self {
        Self {
            enforce_lockfile_check: !cfg!(debug_assertions),
            lockfile_path: PathBuf::from("/tmp/com.leonsilicon.chord-dev.lock"),
        }
    }

    fn init(&self, _: ()) -> anyhow::Result<()> {
        Ok(())
    }
}