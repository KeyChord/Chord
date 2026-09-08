//! Never unlink lease files: replacing an inode would allow two owners.
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::Path;

pub(super) fn is_development(bundle_id: &str) -> bool {
    bundle_id == "com.leonsilicon.chord.development"
        || bundle_id.starts_with("com.leonsilicon.chord.development.")
}

pub(super) struct InputLease {
    owner: File,
    development: File,
    is_development: bool,
    owns_input: bool,
    announced_development: bool,
}

fn open_lock(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

fn try_lock(file: &File, shared: bool) -> io::Result<bool> {
    let result = if shared {
        file.try_lock_shared()
    } else {
        file.try_lock()
    };
    match result {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(std::fs::TryLockError::Error(error)) if error.kind() == ErrorKind::Interrupted => {
            Ok(false)
        }
        Err(std::fs::TryLockError::Error(error)) => Err(error),
    }
}

impl InputLease {
    pub(super) fn open(directory: &Path, is_development: bool) -> io::Result<Self> {
        std::fs::create_dir_all(directory)?;
        Ok(Self {
            owner: open_lock(&directory.join("owner.lock"))?,
            development: open_lock(&directory.join("development.lock"))?,
            is_development,
            owns_input: false,
            announced_development: false,
        })
    }

    /// `changed(false)` quiesces input before unlock. All eligible dev instances
    /// share the priority lock, but only one instance can own input.
    pub(super) fn refresh(
        &mut self,
        eligible: bool,
        mut changed: impl FnMut(bool),
    ) -> io::Result<()> {
        let wants_input = if !eligible {
            false
        } else if self.is_development {
            if !self.announced_development {
                self.announced_development = try_lock(&self.development, true)?;
            }
            self.announced_development
        } else if try_lock(&self.development, false)? {
            self.development.unlock()?;
            true
        } else {
            false
        };
        if self.owns_input && !wants_input {
            changed(false);
            self.owner.unlock()?;
            self.owns_input = false;
        }
        if self.announced_development && !eligible {
            self.development.unlock()?;
            self.announced_development = false;
        }
        if !self.owns_input && wants_input && try_lock(&self.owner, false)? {
            self.owns_input = true;
            changed(true);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "chord-lease-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }
        fn lease(&self, dev: bool) -> InputLease {
            InputLease::open(&self.0, dev).unwrap()
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn refresh(lease: &mut InputLease, eligible: bool) -> Vec<bool> {
        let mut changes = vec![];
        lease
            .refresh(eligible, |active| changes.push(active))
            .unwrap();
        changes
    }
    #[test]
    fn development_preempts_only_after_production_quiesces() {
        let dir = Directory::new();
        let mut prod = dir.lease(false);
        let mut dev = dir.lease(true);
        assert_eq!(refresh(&mut prod, true), [true]);
        assert!(refresh(&mut dev, true).is_empty());
        prod.refresh(true, |active| {
            assert!(!active);
            assert!(refresh(&mut dev, true).is_empty());
        })
        .unwrap();
        assert_eq!(refresh(&mut dev, true), [true]);
        assert!(refresh(&mut prod, true).is_empty());
        drop(dev);
        assert_eq!(refresh(&mut prod, true), [true]);
    }
    #[test]
    fn same_priority_has_one_owner_and_waiting_dev_keeps_priority() {
        let dir = Directory::new();
        let mut dev1 = dir.lease(true);
        let mut dev2 = dir.lease(true);
        let mut prod = dir.lease(false);
        assert_eq!(refresh(&mut dev1, true), [true]);
        assert!(refresh(&mut dev2, true).is_empty());
        assert!(refresh(&mut prod, true).is_empty());
        drop(dev1);
        assert!(refresh(&mut prod, true).is_empty());
        assert_eq!(refresh(&mut dev2, true), [true]);
    }
    #[test]
    fn ineligible_dev_does_not_block_and_permission_loss_releases_both_locks() {
        let dir = Directory::new();
        let mut dev = dir.lease(true);
        let mut prod = dir.lease(false);
        assert!(refresh(&mut dev, false).is_empty());
        assert_eq!(refresh(&mut prod, true), [true]);
        refresh(&mut prod, false);
        assert_eq!(refresh(&mut dev, true), [true]);
        assert_eq!(refresh(&mut dev, false), [false]);
        assert_eq!(refresh(&mut prod, true), [true]);
    }
    #[test]
    fn production_instances_are_exclusive_and_existing_files_are_harmless() {
        let dir = Directory::new();
        let mut first = dir.lease(false);
        let mut second = dir.lease(false);
        assert_eq!(refresh(&mut first, true), [true]);
        assert!(refresh(&mut second, true).is_empty());
        drop(first);
        assert_eq!(refresh(&mut second, true), [true]);
    }
    #[test]
    fn priority_uses_runtime_identity_including_isolated_dev_channels() {
        assert!(is_development("com.leonsilicon.chord.development"));
        assert!(is_development("com.leonsilicon.chord.development.codex"));
        assert!(!is_development("com.leonsilicon.chord"));
        assert!(!is_development("com.leonsilicon.chord.beta"));
        assert!(!is_development("com.leonsilicon.chord.development-other"));
    }
}

#[cfg(test)]
mod process_tests {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    #[test]
    fn lease_child_process() {
        let Some(directory) = std::env::var_os("CHORD_LEASE_TEST_CHILD") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        let mut lease = InputLease::open(&directory, true).unwrap();
        lease.refresh(true, |_| {}).unwrap();
        assert!(lease.owns_input);
        std::fs::write(directory.join("ready"), "ready").unwrap();
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    struct ChildLease {
        process: Child,
        directory: std::path::PathBuf,
    }
    impl Drop for ChildLease {
        fn drop(&mut self) {
            let _ = self.process.kill();
            let _ = self.process.wait();
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }

    #[test]
    fn killed_process_releases_owner_and_priority_without_removing_lock_files() {
        let directory =
            std::env::temp_dir().join(format!("chord-lease-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let process = Command::new(std::env::current_exe().unwrap())
            .args(["lease_child_process", "--nocapture"])
            .env("CHORD_LEASE_TEST_CHILD", &directory)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let mut child = ChildLease { process, directory };
        let deadline = Instant::now() + Duration::from_secs(10);
        while !child.directory.join("ready").exists() {
            assert!(
                child.process.try_wait().unwrap().is_none(),
                "child exited before acquiring locks"
            );
            assert!(Instant::now() < deadline, "child startup timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut prod = InputLease::open(&child.directory, false).unwrap();
        let mut other_dev = InputLease::open(&child.directory, true).unwrap();
        prod.refresh(true, |_| {
            panic!("production acquired while dev owned input")
        })
        .unwrap();
        other_dev
            .refresh(true, |_| panic!("second dev acquired input"))
            .unwrap();
        // Withdraw the second dev's priority request before killing the owner.
        other_dev.refresh(false, |_| {}).unwrap();
        child.process.kill().unwrap();
        child.process.wait().unwrap();
        assert!(child.directory.join("owner.lock").exists());
        assert!(child.directory.join("development.lock").exists());
        let mut acquired = false;
        prod.refresh(true, |active| acquired = active).unwrap();
        assert!(acquired, "kernel did not release crashed process's leases");
    }
}
