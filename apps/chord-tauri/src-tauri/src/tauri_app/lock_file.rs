use anyhow::{Context, Result};
use std::fs::{File, OpenOptions, remove_file};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Runtime};

pub struct AppLockFile {
    inner: Mutex<Option<LockFileState>>,
}

struct LockFileState {
    path: PathBuf,
    _file: File,
}

impl AppLockFile {
    pub fn acquire<R: Runtime>(app: &AppHandle<R>) -> Result<Self> {
        let bundle_id = app.config().identifier.clone();
        let path = lock_file_path(&std::env::temp_dir(), &bundle_id);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .with_context(|| format!("failed to create app lock file at {}", path.display()))?;

        writeln!(file, "pid={}", std::process::id())
            .with_context(|| format!("failed to write app lock file at {}", path.display()))?;
        writeln!(file, "bundle_id={bundle_id}")
            .with_context(|| format!("failed to write app lock file at {}", path.display()))?;
        file.flush()
            .with_context(|| format!("failed to flush app lock file at {}", path.display()))?;

        Ok(Self {
            inner: Mutex::new(Some(LockFileState { path, _file: file })),
        })
    }

    pub fn cleanup(&self) -> Result<()> {
        let state = self
            .inner
            .lock()
            .expect("app lock file mutex poisoned")
            .take();

        let Some(state) = state else {
            return Ok(());
        };

        drop(state._file);
        match remove_file(&state.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).with_context(|| {
                format!("failed to remove app lock file {}", state.path.display())
            }),
        }
    }
}

impl Drop for AppLockFile {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            log::error!("Failed to clean up app lock file: {error}");
        }
    }
}

fn lock_file_path(temp_dir: &Path, bundle_id: &str) -> PathBuf {
    temp_dir.join(format!("{bundle_id}.lock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_runtime_bundle_identifier_for_lock_file_name() {
        let temp_dir = Path::new("/tmp");

        assert_eq!(
            lock_file_path(temp_dir, "com.leonsilicon.chord"),
            Path::new("/tmp/com.leonsilicon.chord.lock")
        );
        assert_eq!(
            lock_file_path(temp_dir, "com.leonsilicon.chord-dev"),
            Path::new("/tmp/com.leonsilicon.chord-dev.lock")
        );
    }
}
