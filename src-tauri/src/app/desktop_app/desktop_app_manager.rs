use anyhow::Result;
use llrt_core::libs::utils::result::ResultExt;
#[allow(unused_imports)]
use log::kv::ToValue;
use objc2_app_kit::{NSRunningApplication, NSWorkspace, NSWorkspaceLaunchOptions};
use objc2_foundation::NSString;
#[allow(unused_imports)]
use rquickjs::{Ctx, Function, Persistent, Promise, Value};
use serde::Serialize;
use std::cell::RefCell;
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use crate::app::desktop_app::DesktopApp;
use crate::app::state::AppSingleton;
use crate::quickjs::with_js;
use crate::state::{DesktopAppManagerObservable, DesktopAppManagerState, Observable};
#[cfg(target_os = "macos")]
use macos::init_macos_observers;

pub struct DesktopAppManager {
    pub(super) observable: DesktopAppManagerObservable,
    pub(super) handle: AppHandle,
}

impl DesktopAppManager {
    #[allow(dead_code)]
    pub fn load_apps_metadata(&self, bundle_ids: &[&str]) -> Result<()> {
        self.observable.try_set_state(|prev| {
            let mut next = prev.clone();
            let mut state = Arc::make_mut(&mut next);

            for bundle_id in bundle_ids {
                let app = DesktopApp::new(&bundle_id)?;
                if let Ok(metadata) = app.get_metadata() {
                    state.apps_metadata.insert(bundle_id.to_string(), metadata);
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
        let bundle_id = bundle_id.to_string();
        let state = self.observable.get_state()?;
        let apps_needing_relaunch = state.apps_needing_relaunch.clone();
        let apps_metadata = state.apps_metadata.clone();
        let mut apps_needing_relaunch = apps_needing_relaunch.clone();

        if needs_relaunch {
            if !apps_needing_relaunch.contains(&bundle_id) {
                apps_needing_relaunch.push(bundle_id);
            }
        } else {
            apps_needing_relaunch.retain(|id| id != &bundle_id);
        }

        self.observable.set_state(DesktopAppManagerState {
            apps_needing_relaunch,
            apps_metadata,
        })?;
        Ok(())
    }
}

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static HAS_LAUNCH_CALLBACKS: AtomicBool = AtomicBool::new(false);
static HAS_TERMINATE_CALLBACKS: AtomicBool = AtomicBool::new(false);
static NEXT_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static APP_LIFECYCLE_CALLBACKS: RefCell<AppLifecycleCallbacks> =
        RefCell::new(AppLifecycleCallbacks::default());
}

#[derive(Default)]
struct AppLifecycleCallbacks {
    launch: Vec<AppLifecycleCallbackEntry>,
    terminate: Vec<AppLifecycleCallbackEntry>,
}

#[derive(Clone)]
struct AppLifecycleCallbackEntry {
    id: u64,
    bundle_id: String,
    callback: Persistent<Function<'static>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedApp {
    pub pid: i32,
    pub bundle_id: String,
}

pub fn init_app_lifecycle(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle.clone());

    #[cfg(target_os = "macos")]
    if let Err(error) = handle.run_on_main_thread(init_macos_observers) {
        log::error!("Failed to initialize app lifecycle observers: {error}");
    }
}

pub fn register_app_launch_handler<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<()> {
    let id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);

    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        callbacks
            .borrow_mut()
            .launch
            .push(AppLifecycleCallbackEntry {
                id,
                bundle_id,
                callback: Persistent::save(&ctx, callback),
            });
    });
    HAS_LAUNCH_CALLBACKS.store(true, Ordering::SeqCst);

    Ok(())
}

pub fn register_app_terminate_handler<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<Function<'js>> {
    let id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);

    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        callbacks
            .borrow_mut()
            .terminate
            .push(AppLifecycleCallbackEntry {
                id,
                bundle_id,
                callback: Persistent::save(&ctx, callback),
            });
    });
    HAS_TERMINATE_CALLBACKS.store(true, Ordering::SeqCst);

    Function::new(ctx.clone(), move || -> rquickjs::Result<()> {
        unregister_app_terminate_handler(id);
        Ok(())
    })?
    .with_name("off")
}

fn unregister_app_terminate_handler(id: u64) {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        callbacks.terminate.retain(|entry| entry.id != id);
        HAS_TERMINATE_CALLBACKS.store(!callbacks.terminate.is_empty(), Ordering::SeqCst);
    });
}

#[allow(dead_code)]
pub fn clear_callbacks() {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        callbacks.launch.clear();
        callbacks.terminate.clear();
    });
    HAS_LAUNCH_CALLBACKS.store(false, Ordering::SeqCst);
    HAS_TERMINATE_CALLBACKS.store(false, Ordering::SeqCst);
}

pub fn dispatch_app_launch(app: ObservedApp) {
    if !HAS_LAUNCH_CALLBACKS.load(Ordering::SeqCst) {
        return;
    }

    let Some(handle) = APP_HANDLE.get().cloned() else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) = with_js(handle, move |ctx| {
            Box::pin(invoke_launch_callbacks(ctx, app))
        })
        .await
        {
            log::error!("Failed to run app launch callbacks: {error}");
        }
    });
}

pub fn dispatch_app_terminate(app: ObservedApp) {
    if !HAS_TERMINATE_CALLBACKS.load(Ordering::SeqCst) {
        return;
    }

    let Some(handle) = APP_HANDLE.get().cloned() else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) = with_js(handle, move |ctx| {
            Box::pin(invoke_terminate_callbacks(ctx, app))
        })
        .await
        {
            log::error!("Failed to run app terminate callbacks: {error}");
        }
    });
}

pub async fn invoke_launch_callbacks<'js>(ctx: Ctx<'js>, app: ObservedApp) -> Result<()> {
    let callbacks = APP_LIFECYCLE_CALLBACKS.with(|callbacks| callbacks.borrow().launch.clone());

    for callback in callbacks {
        if callback.bundle_id != app.bundle_id {
            continue;
        }

        let callback = callback.callback.restore(&ctx)?;
        let js_app = rquickjs_serde::to_value(ctx.clone(), app.clone())
            .or_throw_msg(&ctx, "failed to serialize launch payload")?;
        let result: Value<'js> = callback.call((js_app,))?;
        if let Some(promise) = result.into_promise() {
            promise.into_future::<Value<'js>>().await.map(|_| ())?
        }
    }

    Ok(())
}

pub async fn invoke_terminate_callbacks<'js>(ctx: Ctx<'js>, app: ObservedApp) -> Result<()> {
    let callbacks = APP_LIFECYCLE_CALLBACKS.with(|callbacks| callbacks.borrow().terminate.clone());

    for callback in callbacks {
        if callback.bundle_id != app.bundle_id {
            continue;
        }

        let callback = callback.callback.restore(&ctx)?;
        let js_app = rquickjs_serde::to_value(ctx.clone(), app.clone())
            .or_throw_msg(&ctx, "failed to serialize app terminate payload")?;
        let result: Value<'js> = callback.call((js_app,))?;
        if let Some(promise) = result.into_promise() {
            promise.into_future::<Value<'js>>().await.map(|_| ())?
        }
    }

    Ok(())
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

