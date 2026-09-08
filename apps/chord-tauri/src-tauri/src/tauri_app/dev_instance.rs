//! Development launches replace the previous app before Tauri's single-instance check.
//! Keep the plugin's socket naming in sync with tauri-plugin-single-instance (macOS).
use anyhow::{Context, Result, bail};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub fn should_replace(identifier: &str, has_cli_command: bool) -> bool {
    !has_cli_command
        && (identifier == "com.leonsilicon.chord.development"
            || identifier.starts_with("com.leonsilicon.chord.development."))
}

pub struct LaunchGuard {
    _lock: File,
    socket: PathBuf,
}

impl LaunchGuard {
    pub fn acquire(identifier: &str) -> Result<Self> {
        let name = identifier.replace(['.', '-'], "_");
        let socket = PathBuf::from(format!("/tmp/{name}_si.sock"));
        // This inode must persist: unlinking a lockfile lets concurrent launches lock
        // different files. The OS releases the lock when this guard/process exits.
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(format!("/tmp/{name}_dev_launch.lock"))?;
        lock.lock()
            .context("failed to serialize development launches")?;
        let guard = Self {
            _lock: lock,
            socket,
        };
        if let Some(pid) = guard.peer_pid()? {
            eprintln!("Replacing previous Chord development instance (PID {pid})");
            // Obtain the PID from the live socket, never from a stale PID file or
            // a process-name match that could target another app/channel.
            if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error).context("failed to stop previous development instance");
                }
            }
            let deadline = Instant::now() + Duration::from_secs(10);
            let force_after = Instant::now() + Duration::from_secs(2);
            let mut forced = false;
            loop {
                if unsafe { libc::kill(pid, 0) } != 0
                    && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    break;
                }
                if !forced && Instant::now() >= force_after {
                    if guard.peer_pid()? != Some(pid) {
                        bail!("previous development instance changed before it exited");
                    }
                    eprintln!("Previous development instance ignored SIGTERM; stopping PID {pid}");
                    if unsafe { libc::kill(pid, libc::SIGKILL) } != 0 {
                        let error = io::Error::last_os_error();
                        if error.raw_os_error() != Some(libc::ESRCH) {
                            return Err(error).context("failed to force-stop development instance");
                        }
                    }
                    forced = true;
                }
                if Instant::now() >= deadline {
                    bail!(
                        "previous development instance (PID {pid}) did not exit within 10 seconds"
                    );
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        Ok(guard)
    }

    // The plugin binds asynchronously. Do not admit the next launch until this
    // process is discoverable, or both launches could pass the singleton check.
    pub fn wait_until_listening(&self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.peer_pid()? != Some(std::process::id() as libc::pid_t) {
            if Instant::now() >= deadline {
                bail!("development instance did not start its single-instance listener");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    }

    fn peer_pid(&self) -> Result<Option<libc::pid_t>> {
        let stream = match UnixStream::connect(&self.socket) {
            Ok(stream) => stream,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error).context("failed to connect to development instance"),
        };
        let mut pid: libc::pid_t = 0;
        let mut size = std::mem::size_of_val(&pid) as libc::socklen_t;
        let result = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_LOCAL,
                libc::LOCAL_PEERPID,
                (&mut pid as *mut libc::pid_t).cast(),
                &mut size,
            )
        };
        let socket_error = (result != 0).then(io::Error::last_os_error);
        // Avoid sending an empty notification to the existing app's callback.
        // The plugin ignores invalid UTF-8 rather than dispatching it.
        use std::io::Write;
        let _ = (&stream).write_all(&[0xff]);
        if let Some(error) = socket_error {
            return Err(error).context("failed to identify development instance");
        }
        if pid <= 1 {
            bail!("invalid development instance PID {pid}");
        }
        Ok(Some(pid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_fixture() {
        let Ok(socket) = std::env::var("CHORD_TEST_INSTANCE_SOCKET") else {
            return;
        };
        let _listener = std::os::unix::net::UnixListener::bind(socket).unwrap();
        if std::env::var_os("CHORD_TEST_IGNORE_TERM").is_some() {
            unsafe {
                libc::signal(libc::SIGTERM, libc::SIG_IGN);
            }
        }
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    #[test]
    fn replaces_live_socket_owner_and_accepts_stale_socket() {
        check_replacement(false);
        check_replacement(true);
    }

    fn check_replacement(ignore_term: bool) {
        let id = format!(
            "com.leonsilicon.chord.development.test{}",
            std::process::id()
        );
        let name = id.replace('.', "_");
        let socket = format!("/tmp/{name}_si.sock");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tauri_app::dev_instance::tests::instance_fixture",
            ])
            .env("CHORD_TEST_INSTANCE_SOCKET", &socket)
            .envs(ignore_term.then_some(("CHORD_TEST_IGNORE_TERM", "1")))
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !std::path::Path::new(&socket).exists() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("fixture did not bind socket");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Reap the child concurrently, as kill(pid, 0) also sees zombie processes.
        let reaper = std::thread::spawn(move || child.wait().unwrap());
        let guard = LaunchGuard::acquire(&id).unwrap();
        assert!(!reaper.join().unwrap().success());
        drop(guard);
        drop(LaunchGuard::acquire(&id).unwrap());
        std::fs::remove_file(socket).unwrap();
        std::fs::remove_file(format!("/tmp/{name}_dev_launch.lock")).unwrap();
    }

    #[test]
    fn replaces_only_development_app_launches() {
        for id in [
            "com.leonsilicon.chord.development",
            "com.leonsilicon.chord.development.codex",
        ] {
            assert!(should_replace(id, false));
            assert!(!should_replace(id, true));
        }
        for id in [
            "com.leonsilicon.chord",
            "com.leonsilicon.chord.beta",
            "com.leonsilicon.chord.development-other",
        ] {
            assert!(!should_replace(id, false));
        }
    }
}
