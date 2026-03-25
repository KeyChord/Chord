use std::path::{Path, PathBuf};

pub struct DevLockfileDetector {
    lockfile_path: PathBuf,
}

impl DevLockfileDetector {
    pub fn new() -> Self {
        Self {
            lockfile_path: PathBuf::from("/tmp/com.leonsilicon.chords-dev.lock"),
        }
    }

    pub fn should_intercept_input_events(&self) -> bool {
        !is_dev_lockfile_present(&self.lockfile_path)
    }
}

fn is_dev_lockfile_present(lockfile_path: &Path) -> bool {
    lockfile_path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disables_interception_when_lockfile_exists() {
        let temp_path =
            std::env::temp_dir().join(format!("dev-lockfile-detector-test-{}", std::process::id()));
        std::fs::write(&temp_path, b"locked").unwrap();

        assert!(is_dev_lockfile_present(&temp_path));

        std::fs::remove_file(&temp_path).unwrap();
    }

    #[test]
    fn enables_interception_when_lockfile_is_missing() {
        let temp_path = std::env::temp_dir().join(format!(
            "dev-lockfile-detector-missing-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&temp_path);

        assert!(!is_dev_lockfile_present(&temp_path));
    }
}
