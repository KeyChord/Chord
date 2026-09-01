//! App launch / terminate callbacks registered from JS through the `chord` module.

use crate::app::desktop_app::ObservedApp;
use crate::bun_js::with_js;
use anyhow::Result;
use rbun::Persistent;
use rbun::prelude::*;
use std::cell::RefCell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::AppHandle;
use tokio::sync::Semaphore;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static HAS_LAUNCH_CALLBACKS: AtomicBool = AtomicBool::new(false);
static HAS_TERMINATE_CALLBACKS: AtomicBool = AtomicBool::new(false);
static NEXT_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);
static REGISTRATION_GENERATION: AtomicU64 = AtomicU64::new(0);
static LIFECYCLE_DISPATCH_SLOTS: Semaphore = Semaphore::const_new(64);
static LIFECYCLE_DROPPED_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);

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
    generation: u64,
    bundle_id: String,
    callback: Persistent<Function<'static>>,
}

pub fn init_app_lifecycle(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle.clone());
    // The macOS workspace observers are installed by the desktop-app manager
    // for whichever engine is active.
    crate::app::desktop_app::init_app_lifecycle(handle);
}

pub fn begin_registration_generation() -> u64 {
    REGISTRATION_GENERATION.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn retain_registration_generation(generation: u64) {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        callbacks
            .launch
            .retain(|entry| entry.generation == generation);
        callbacks
            .terminate
            .retain(|entry| entry.generation == generation);
        HAS_LAUNCH_CALLBACKS.store(!callbacks.launch.is_empty(), Ordering::SeqCst);
        HAS_TERMINATE_CALLBACKS.store(!callbacks.terminate.is_empty(), Ordering::SeqCst);
    });
}

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
                generation: REGISTRATION_GENERATION.load(Ordering::SeqCst),
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
) -> rbun::Result<Function<'js>> {
    let id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);

    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        callbacks
            .borrow_mut()
            .terminate
            .push(AppLifecycleCallbackEntry {
                id,
                generation: REGISTRATION_GENERATION.load(Ordering::SeqCst),
                bundle_id,
                callback: Persistent::save(&ctx, callback),
            });
    });
    HAS_TERMINATE_CALLBACKS.store(true, Ordering::SeqCst);

    Function::new(ctx.clone(), move || -> rbun::Result<()> {
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
    let Ok(permit) = LIFECYCLE_DISPATCH_SLOTS.try_acquire() else {
        warn_lifecycle_event_dropped();
        return;
    };

    tauri::async_runtime::spawn(async move {
        let _permit = permit;
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
    let Ok(permit) = LIFECYCLE_DISPATCH_SLOTS.try_acquire() else {
        warn_lifecycle_event_dropped();
        return;
    };

    tauri::async_runtime::spawn(async move {
        let _permit = permit;
        if let Err(error) = with_js(handle, move |ctx| {
            Box::pin(invoke_terminate_callbacks(ctx, app))
        })
        .await
        {
            log::error!("Failed to run app terminate callbacks: {error}");
        }
    });
}

fn warn_lifecycle_event_dropped() {
    if !LIFECYCLE_DROPPED_WARNING_EMITTED.swap(true, Ordering::Relaxed) {
        log::warn!("Dropping app lifecycle events because the bounded Bun queue is saturated");
    }
}

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
