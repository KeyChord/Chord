//! App launch / terminate callbacks registered from JS through the `chord` module.

use crate::app::desktop_app::ObservedApp;
use crate::bun_js::package_worker;
#[cfg(target_os = "macos")]
use anyhow::Result;
#[cfg(target_os = "macos")]
use rbun::Persistent;
#[cfg(target_os = "macos")]
use rbun::prelude::*;
#[cfg(target_os = "macos")]
use std::cell::RefCell;
#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use tokio::sync::Semaphore;

#[cfg(target_os = "macos")]
static NEXT_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);
static LIFECYCLE_DISPATCH_SLOTS: Semaphore = Semaphore::const_new(64);
static LIFECYCLE_DROPPED_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
thread_local! {
    static APP_LIFECYCLE_CALLBACKS: RefCell<AppLifecycleCallbacks> =
        RefCell::new(AppLifecycleCallbacks::default());
}

#[cfg(target_os = "macos")]
#[derive(Default)]
struct AppLifecycleCallbacks {
    launch: Vec<AppLifecycleCallbackEntry>,
    terminate: Vec<AppLifecycleCallbackEntry>,
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct AppLifecycleCallbackEntry {
    id: u64,
    bundle_id: String,
    callback: Persistent<Function<'static>>,
}

/// Whether the package in this worker listens for apps launching or quitting.
#[cfg(target_os = "macos")]
pub fn has_callbacks() -> bool {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        let callbacks = callbacks.borrow();
        !callbacks.launch.is_empty() || !callbacks.terminate.is_empty()
    })
}

pub fn init_app_lifecycle(handle: AppHandle) {
    // The macOS workspace observers are installed by the desktop-app manager
    // for whichever engine is active.
    crate::app::desktop_app::init_app_lifecycle(handle);
}

#[cfg(target_os = "macos")]
pub fn register_app_launch_handler<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rbun::Result<()> {
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

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn register_app_terminate_handler<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rbun::Result<Function<'js>> {
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

    Function::new(ctx.clone(), move || -> rbun::Result<()> {
        unregister_app_terminate_handler(id);
        Ok(())
    })?
    .with_name("off")
}

#[cfg(target_os = "macos")]
fn unregister_app_terminate_handler(id: u64) {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        callbacks.terminate.retain(|entry| entry.id != id);
    });
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn clear_callbacks() {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        callbacks.launch.clear();
        callbacks.terminate.clear();
    });
}

pub fn dispatch_app_launch(app: ObservedApp) {
    dispatch(app, true);
}

pub fn dispatch_app_terminate(app: ObservedApp) {
    dispatch(app, false);
}

fn dispatch(app: ObservedApp, launch: bool) {
    let Ok(permit) = LIFECYCLE_DISPATCH_SLOTS.try_acquire() else {
        warn_lifecycle_event_dropped();
        return;
    };
    tauri::async_runtime::spawn(async move {
        let _permit = permit;
        package_worker::dispatch_lifecycle(app, launch).await;
    });
}

fn warn_lifecycle_event_dropped() {
    if !LIFECYCLE_DROPPED_WARNING_EMITTED.swap(true, Ordering::Relaxed) {
        log::warn!("Dropping app lifecycle events because the bounded Bun queue is saturated");
    }
}

#[cfg(target_os = "macos")]
pub async fn invoke_launch_callbacks<'js>(ctx: Ctx<'js>, app: ObservedApp) -> Result<()> {
    let callbacks = APP_LIFECYCLE_CALLBACKS.with(|callbacks| callbacks.borrow().launch.clone());

    for callback in callbacks {
        if callback.bundle_id != app.bundle_id {
            continue;
        }

        let callback = callback.callback.restore(&ctx)?;
        let js_app = rbun::serde::to_value(ctx.clone(), app.clone())
            .or_throw_msg(&ctx, "failed to serialize launch payload")?;
        let result: Value<'js> = callback.call((js_app,))?;
        if let Some(promise) = result.into_promise() {
            promise.into_future::<Value<'js>>().await.map(|_| ())?
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub async fn invoke_terminate_callbacks<'js>(ctx: Ctx<'js>, app: ObservedApp) -> Result<()> {
    let callbacks = APP_LIFECYCLE_CALLBACKS.with(|callbacks| callbacks.borrow().terminate.clone());

    for callback in callbacks {
        if callback.bundle_id != app.bundle_id {
            continue;
        }

        let callback = callback.callback.restore(&ctx)?;
        let js_app = rbun::serde::to_value(ctx.clone(), app.clone())
            .or_throw_msg(&ctx, "failed to serialize app terminate payload")?;
        let result: Value<'js> = callback.call((js_app,))?;
        if let Some(promise) = result.into_promise() {
            promise.into_future::<Value<'js>>().await.map(|_| ())?
        }
    }

    Ok(())
}
