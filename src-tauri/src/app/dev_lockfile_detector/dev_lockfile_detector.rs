use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct DevLockfileDetector {
    pub(super) enforce_lockfile_check: bool,
    pub(super) lockfile_path: PathBuf,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn detector_for_test(
        lockfile_path: PathBuf,
        enforce_lockfile_check: bool,
    ) -> DevLockfileDetector {
        DevLockfileDetector {
            enforce_lockfile_check,
            lockfile_path,
        }
    }

    #[test]
    fn disables_interception_when_lockfile_exists() {
        let temp_path =
            std::env::temp_dir().join(format!("dev-lockfile-detector-test-{}", std::process::id()));
        std::fs::write(&temp_path, b"locked").unwrap();

        let detector = detector_for_test(temp_path.clone(), true);

        assert!(!detector.should_intercept_input_events());

        std::fs::remove_file(&temp_path).unwrap();
    }

    #[test]
    fn enables_interception_when_lockfile_is_missing() {
        let temp_path = std::env::temp_dir().join(format!(
            "dev-lockfile-detector-missing-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&temp_path);

        let detector = detector_for_test(temp_path, true);

        assert!(detector.should_intercept_input_events());
    }

    #[test]
    fn ignores_lockfile_when_check_is_disabled() {
        let temp_path = std::env::temp_dir().join(format!(
            "dev-lockfile-detector-disabled-test-{}",
            std::process::id()
        ));
        std::fs::write(&temp_path, b"locked").unwrap();

        let detector = detector_for_test(temp_path.clone(), false);

        assert!(detector.should_intercept_input_events());

        std::fs::remove_file(&temp_path).unwrap();
    }
}
