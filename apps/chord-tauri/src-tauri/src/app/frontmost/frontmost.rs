use crate::app::AppHandleExt;
use crate::app::desktop_app::AppIdentity;
use crate::app::state::AppSingleton;
use crate::state::{FrontmostObservable, FrontmostState, Observable};
use anyhow::Result;
#[cfg(target_os = "macos")]
use frontmost::{Detector, start_nsrunloop};
use nject::injectable;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSWorkspace;
use observable_property::ObservableProperty;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;
use tauri::AppHandle;

#[derive(Debug)]
pub(super) struct FrontmostTracker {
    pub(super) handle: AppHandle,
}

#[cfg(target_os = "macos")]
impl frontmost::app::FrontmostApp for FrontmostTracker {
    /// Runs on the AppKit main thread from an NSWorkspace notification callback, so it must
    /// never panic: unwinding out of an Objective-C callback is undefined behavior.
    fn set_frontmost(&mut self, bundle_id: Option<String>) {
        // The crate reports only bundle IDs. This runs while the activation is handled on the
        // main thread, so the workspace's frontmost app is the activated one, and it also
        // identifies bare executables, which have no bundle ID.
        let identity = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| AppIdentity::of(&app))
            .filter(|identity| bundle_id.is_none() || identity.declared_id == bundle_id)
            .unwrap_or_else(|| AppIdentity::from_bundle_id(bundle_id));
        let frontmost = self.handle.app_state().frontmost();
        if let Err(e) = frontmost.set_frontmost(identity) {
            log::error!("failed to set frontmost app: {e}");
        }
    }

    /// See [`Self::set_frontmost`] — must never panic.
    fn update(&mut self) {
        let frontmost = self.handle.app_state().frontmost();
        match frontmost.frontmost() {
            Ok(app) => log::debug!("Application activated: {app:?}"),
            Err(e) => log::error!("failed to read frontmost app: {e}"),
        }
    }
}

#[injectable]
#[derive(Debug)]
pub struct AppFrontmost {
    /// Needs to be shared with `FrontmostTracker
    observable: FrontmostObservable,
    handle: AppHandle,
    /// Kept to re-resolve bare executables when packages change their `executables` claims.
    #[inject(Mutex::new(None))]
    identity: Mutex<Option<AppIdentity>>,
}

impl AppFrontmost {
    #[cfg(target_os = "macos")]
    pub(super) fn init(&self) -> Result<()> {
        let workspace = NSWorkspace::sharedWorkspace();
        if let Some(application) = workspace.frontmostApplication() {
            self.set_frontmost(AppIdentity::of(&application))?;
        }

        thread::spawn(|| {
            start_nsrunloop!();
        });

        Detector::init(Box::new(FrontmostTracker {
            handle: self.handle.clone(),
        }));

        #[cfg(target_os = "macos")]
        super::file_picker::init(self.handle.clone())?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn init(&self) -> Result<()> {
        let handle = self.handle.clone();
        thread::Builder::new()
            .name("chord-frontmost".into())
            .spawn(move || {
                let mut previous = None;
                loop {
                    let active = crate::desktop_platform::foreground_app();
                    if let Some(app) = active.as_ref().filter(|app| app.pid != std::process::id()) {
                        if previous.as_ref() != Some(app) {
                            let identity = AppIdentity::from_process(app);
                            if let Err(error) =
                                handle.app_state().frontmost().set_frontmost(identity)
                            {
                                log::warn!("Could not publish focused app: {error:#}");
                            }
                            previous = active;
                        }
                    }
                    thread::sleep(std::time::Duration::from_millis(80));
                }
            })?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn capture_current(&self) {
        if let Some(app) = crate::desktop_platform::foreground_app()
            && app.pid != std::process::id()
            && let Err(error) = self.set_frontmost(AppIdentity::from_process(&app))
        {
            log::warn!("Could not resolve the foreground app: {error:#}");
        }
    }

    pub fn frontmost(&self) -> Result<Option<String>> {
        Ok(self.observable.get_state()?.frontmost_app_bundle_id)
    }

    pub fn refresh_chord_context(&self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let overlay = crate::native_input::screenshot_markup_visible()
                .then(|| "com.apple.screencaptureui".to_owned());
            if self.observable.get_state()?.overlay_app_bundle_id != overlay {
                self.observable.set_state(|state| FrontmostState {
                    overlay_app_bundle_id: overlay,
                    ..state
                })?;
            }
        }
        Ok(())
    }

    pub fn chord_application(&self) -> Result<Option<String>> {
        Ok(self
            .observable
            .get_state()?
            .chord_application()
            .map(str::to_owned))
    }

    pub fn chord_file_picker_active(&self) -> bool {
        self.observable
            .get_state()
            .is_ok_and(|state| state.chord_file_picker_active())
    }

    pub(super) fn set_frontmost(&self, identity: AppIdentity) -> Result<()> {
        // Resolve and publish under the lock so a package reload cannot publish a stale app.
        let mut current = self.identity.lock();
        let app_id = identity.resolve(&self.handle);
        *current = Some(identity);
        self.set_app_id(app_id)
    }

    /// Re-resolves the frontmost app after packages change which executables they claim.
    pub fn refresh(&self) -> Result<()> {
        let current = self.identity.lock();
        let Some(identity) = current.as_ref() else {
            return Ok(());
        };
        let app_id = identity.resolve(&self.handle);
        if self.frontmost()? == app_id {
            return Ok(());
        }
        self.set_app_id(app_id)
    }

    fn set_app_id(&self, app_id: Option<String>) -> Result<()> {
        self.observable.set_state(|previous| FrontmostState {
            file_picker_active: previous.file_picker_active
                && previous.frontmost_app_bundle_id == app_id,
            frontmost_app_bundle_id: app_id,
            ..previous
        })
    }

    pub fn file_picker_active(&self) -> bool {
        self.observable
            .get_state()
            .is_ok_and(|state| state.file_picker_active)
    }

    pub(super) fn set_file_picker_active(&self, active: bool) -> Result<()> {
        if self.file_picker_active() == active {
            return Ok(());
        }
        log::debug!("File picker context: {active}");
        self.observable.set_state(|state| FrontmostState {
            file_picker_active: active,
            ..state
        })
    }
}
