use nject::injectable;
use std::path::{Path, PathBuf};

#[injectable]
#[allow(dead_code)]
pub struct DevLockfileDetector {
    #[inject(false)]
    enforce_lockfile_check: bool,
    #[inject(PathBuf::new())]
    lockfile_path: PathBuf,
}

impl DevLockfileDetector {
    #[allow(dead_code)]
    pub fn should_intercept_input_events(&self) -> bool {
        if !self.enforce_lockfile_check {
            return true;
        }

        !is_dev_lockfile_present(&self.lockfile_path)
    }
}

#[allow(dead_code)]
fn is_dev_lockfile_present(lockfile_path: &Path) -> bool {
    lockfile_path.exists()
}
