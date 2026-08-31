use crate::app::desktop_app::DesktopApp;
use crate::state::{DesktopAppManagerObservable, DesktopAppManagerState, Observable};
use anyhow::Result;
#[cfg(target_os = "macos")]
use macos::init_macos_observers;
use nject::injectable;
use objc2_app_kit::{NSRunningApplication, NSWorkspace, NSWorkspaceLaunchOptions};
use objc2_foundation::NSString;
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::AppHandle;

#[injectable]
pub struct DesktopAppManager {
    observable: DesktopAppManagerObservable,
}

impl DesktopAppManager {
    #[allow(dead_code)]
    pub fn load_apps_metadata(&self, bundle_ids: &[&str]) -> Result<()> {
        self.observable.try_set_state(|prev| {
            let mut next = prev;

            for bundle_id in bundle_ids {
                let app = DesktopApp::new(&bundle_id)?;
                if let Ok(metadata) = app.get_metadata() {
                    next.apps_metadata.insert(bundle_id.to_string(), metadata);
                }
            }

            Ok(next)
        })?;
        Ok(())
    }

    pub fn relaunch_app(&self, bundle_id: &str) -> Result<()> {
        let _app = DesktopApp::new(bundle_id)?;

        let bundle_id = NSString::from_str(bundle_id);
        let running_apps =
            NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);

        for app in running_apps.iter() {
            app.terminate();
        }

        // NSRunningApplication::terminate is async, so give the app a brief window to exit
        // before asking LaunchServices to start it again.
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let still_running =
                NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);
            if still_running.is_empty() {
                break;
            }

            std::thread::sleep(Duration::from_millis(50));
        }

        let workspace = NSWorkspace::sharedWorkspace();
        #[allow(deprecated)]
        let launched = workspace
            .launchAppWithBundleIdentifier_options_additionalEventParamDescriptor_launchIdentifier(
                &bundle_id,
                NSWorkspaceLaunchOptions::Default,
                None,
                None,
            );

        if !launched {
            anyhow::bail!("failed to relaunch app with bundle id {}", bundle_id);
        }

        Ok(())
    }

    pub fn set_app_needs_relaunch(&self, bundle_id: &str, needs_relaunch: bool) -> Result<()> {
        self.observable.set_state(|prev| {
            let bundle_id = bundle_id.to_string();
            let mut apps_needing_relaunch = prev.apps_needing_relaunch.clone();

            if needs_relaunch {
                if !apps_needing_relaunch.contains(&bundle_id) {
                    apps_needing_relaunch.push(bundle_id);
                }
            } else {
                apps_needing_relaunch.retain(|id| id != &bundle_id);
            }
            DesktopAppManagerState {
                apps_needing_relaunch,
                ..prev
            }
        })?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedApp {
    pub pid: i32,
    pub bundle_id: String,
}

pub fn init_app_lifecycle(handle: AppHandle) {
    #[cfg(target_os = "macos")]
    if let Err(error) = handle.run_on_main_thread(init_macos_observers) {
        log::error!("Failed to initialize app lifecycle observers: {error}");
    }
}

pub fn dispatch_app_launch(app: ObservedApp) {
    crate::bun_js::lifecycle::dispatch_app_launch(app);
}

pub fn dispatch_app_terminate(app: ObservedApp) {
    crate::bun_js::lifecycle::dispatch_app_terminate(app);
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{ObservedApp, dispatch_app_launch, dispatch_app_terminate};
    use block2::RcBlock;
    use core::ptr::NonNull;
    use objc2::MainThreadMarker;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{
        NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
        NSWorkspaceDidLaunchApplicationNotification,
        NSWorkspaceDidTerminateApplicationNotification,
    };
    use objc2_foundation::NSNotification;
    use std::sync::OnceLock;

    static OBSERVERS_INITIALIZED: OnceLock<()> = OnceLock::new();

    pub fn init_macos_observers() {
        if OBSERVERS_INITIALIZED.set(()).is_err() {
            return;
        }

        let _main_thread = MainThreadMarker::new()
            .expect("app lifecycle observers must initialize on the main thread");

        let workspace = NSWorkspace::sharedWorkspace();
        let center = workspace.notificationCenter();

        let launch_block = Box::leak(Box::new(RcBlock::new(|notification| {
            if let Some(app) = observed_app_from_notification(notification) {
                dispatch_app_launch(app);
            }
        })));
        let terminate_block = Box::leak(Box::new(RcBlock::new(|notification| {
            if let Some(app) = observed_app_from_notification(notification) {
                dispatch_app_terminate(app);
            }
        })));

        let launch_observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceDidLaunchApplicationNotification),
                None::<&AnyObject>,
                None,
                launch_block,
            )
        };
        let terminate_observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceDidTerminateApplicationNotification),
                None::<&AnyObject>,
                None,
                terminate_block,
            )
        };

        let _ = Box::leak(Box::new(launch_observer));
        let _ = Box::leak(Box::new(terminate_observer));
    }

    fn observed_app_from_notification(
        notification: NonNull<NSNotification>,
    ) -> Option<ObservedApp> {
        let notification = unsafe { notification.as_ref() };

        let Some(user_info) = notification.userInfo() else {
            return None;
        };

        let application_key = unsafe { NSWorkspaceApplicationKey };
        let Some(app_obj) = user_info.objectForKey(application_key) else {
            return None;
        };

        let Ok(app) = app_obj.downcast::<NSRunningApplication>() else {
            return None;
        };

        let Some(bundle_id) = app.bundleIdentifier() else {
            return None;
        };

        let pid = app.processIdentifier();
        if pid <= 0 {
            return None;
        }

        Some(ObservedApp {
            pid,
            bundle_id: bundle_id.to_string(),
        })
    }
}
