use crate::state::{AppPermissionsObservable, AppPermissionsState, Observable};
use anyhow::{Context, Result};
use nject::injectable;
use std::sync::mpsc;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

#[injectable]
pub struct AppPermissions {
    observable: AppPermissionsObservable,
    handle: AppHandle,
    #[inject(ExecutableIdentity::capture())]
    executable: ExecutableIdentity,
}

/// Accessibility is the only permission Chord's input needs: it covers the active
/// event tap, and macOS counts it as Input Monitoring too (`IOHIDCheckAccess` reports
/// listen access granted for an app that only has Accessibility).
#[derive(Debug, Clone, Copy)]
pub struct InputPermissions {
    pub accessibility: bool,
}

impl InputPermissions {
    pub const UNAVAILABLE: Self = Self {
        accessibility: false,
    };
}

struct ExecutableIdentity {
    path: Option<std::path::PathBuf>,
    #[cfg(unix)]
    inode: Option<(u64, u64)>,
}

impl ExecutableIdentity {
    fn capture() -> Self {
        let path = std::env::current_exe().ok();
        #[cfg(unix)]
        let inode = path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|metadata| {
                use std::os::unix::fs::MetadataExt;
                (metadata.dev(), metadata.ino())
            });
        Self {
            path,
            #[cfg(unix)]
            inode,
        }
    }

    fn invalidation(&self) -> Option<String> {
        if let Err(error) = std::env::current_exe() {
            return Some(format!(
                "Running executable path is unavailable ({error}); a rebuild may have unlinked it. Restart Chord from the development snapshot runner"
            ));
        }
        let path = self.path.as_ref()?;
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return Some(format!(
                    "Running executable disappeared at {path:?}: {error}; restart Chord"
                ));
            }
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if self
                .inode
                .is_some_and(|previous| previous != (metadata.dev(), metadata.ino()))
            {
                return Some(format!(
                    "Running executable was replaced at {path:?}; restart Chord from the development snapshot runner"
                ));
            }
        }
        None
    }
}

impl AppPermissions {
    pub(super) fn init(&self) -> Result<()> {
        // The input supervisor (`AppKeyboard`) polls `refresh_if_changed` and
        // owns listener registration; nothing to subscribe to here.
        Ok(())
    }

    pub fn can_intercept_input(&self) -> bool {
        self.observable
            .get_state()
            .is_ok_and(|state| state.is_accessibility_enabled == Some(true))
    }

    pub fn check_now() -> InputPermissions {
        InputPermissions {
            accessibility: macos_accessibility_client::accessibility::application_is_trusted(),
        }
    }

    /// Re-checks the permission and publishes it only when it changed.
    /// Cheap enough to poll: a local TCC lookup.
    pub fn refresh_if_changed(&self) -> Result<InputPermissions> {
        let permissions = Self::check_now();
        let accessibility = permissions.accessibility;
        let previous = self.observable.get_state()?;
        if previous.is_accessibility_enabled != Some(accessibility) {
            log::info!(
                "Permissions: accessibility={accessibility} (executable {:?})",
                std::env::current_exe().ok()
            );
            if !accessibility && let Some(reason) = self.executable.invalidation() {
                log::warn!("{reason}");
            }
            self.observable.set_state(|prev| AppPermissionsState {
                is_accessibility_enabled: Some(accessibility),
                ..prev
            })?;
        }
        Ok(permissions)
    }

    pub async fn load(&self) -> Result<bool> {
        match self.handle.autolaunch().is_enabled() {
            Ok(enabled) => self.observable.set_state(|prev| AppPermissionsState {
                is_autostart_enabled: Some(enabled),
                ..prev
            })?,
            Err(error) => log::warn!("Could not read the launch on login setting: {error:#}"),
        }
        Ok(self.refresh_if_changed()?.accessibility)
    }

    pub fn toggle_autostart(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        let Some(is_autostart_enabled) = state.is_autostart_enabled else {
            anyhow::bail!("app not ready")
        };
        if is_autostart_enabled {
            self.handle.autolaunch().disable()?;
            self.observable.set_state(|prev| AppPermissionsState {
                is_autostart_enabled: Some(false),
                ..prev
            })?;
        } else {
            self.handle.autolaunch().enable()?;
            self.observable.set_state(|prev| AppPermissionsState {
                is_autostart_enabled: Some(true),
                ..prev
            })?;
        }
        Ok(())
    }
}

pub fn open_system_settings(url: &str, permission_name: &str) {
    if let Err(error) = std::process::Command::new("open").arg(url).spawn() {
        log::error!("Failed to open {permission_name} settings: {error}");
    }
}

fn run_on_main_thread_sync<R: Runtime, F>(handle: &AppHandle<R>, f: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    let (tx, rx) = mpsc::sync_channel(0);
    handle
        .run_on_main_thread(move || {
            f();
            let _ = tx.send(());
        })
        .context("schedule main-thread dispatch")?;
    rx.recv()
        .context("main thread finished without signaling (channel closed)")?;
    Ok(())
}

/// `AXIsProcessTrustedWithOptions` must run on the main thread; otherwise the prompt can appear but
/// TCC may not associate the app correctly with the bundle UI in System Settings.
#[cfg(target_os = "macos")]
pub fn request_accessibility_prompt_sync<R: Runtime>(handle: &AppHandle<R>) -> Result<()> {
    log::info!(
        "Requesting accessibility trust for executable {:?}",
        std::env::current_exe().ok()
    );
    run_on_main_thread_sync(handle, || {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    })
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility_prompt_sync<R: Runtime>(_handle: &AppHandle<R>) -> Result<()> {
    Ok(())
}
