use std::borrow::BorrowMut;
use crate::app::state::StateSingleton;
use crate::app::chord_runner::{ChordActionTaskRunner};
use crate::app::chorder::AppChorder;
use crate::app::context::AppContext;
use crate::app::desktop_app::DesktopAppManager;
use crate::app::dev_lockfile_detector::DevLockfileDetector;
use crate::app::frontmost::AppFrontmost;
use crate::app::git_repos_store::GitReposStore;
use crate::app::global_hotkey_store::GlobalHotkeyStore;
use crate::app::permissions::AppPermissions;
use crate::app::placeholder_chord_store::PlaceholderChordStore;
use crate::app::settings::AppSettings;
use crate::app::{AppHandleExt, AppManaged};
use crate::lock_file::AppLockFile;
use crate::observables::{AppPermissionsObservable, AppSettingsObservable, ChordPackageManagerObservable, ChorderObservable, DesktopAppManagerObservable, FrontmostObservable, GitReposObservable, Observable};
use crate::tauri_app;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use crate::app::chord_package_manager::ChordPackageManager;

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
pub fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    let handle = app.handle().clone();
    let app_lock_file = AppLockFile::acquire(app.handle())?;
    app.handle().manage(app_lock_file);

    unsafe {
        let s = (
            singleton(handle.clone(), AppFrontmost::new(handle.clone())),
            singleton(handle.clone(), AppChorder::new(handle.clone())),
            singleton(handle.clone(), AppContext::new(handle.clone())),
            singleton(handle.clone(), DevLockfileDetector::new()),
            singleton(handle.clone(), AppPermissions::new(handle.clone())),
            singleton(handle.clone(), AppSettings::new(handle.clone())),
            singleton(handle.clone(), GlobalHotkeyStore::new(handle.clone())),
            singleton(handle.clone(), PlaceholderChordStore::new(handle.clone())),
            singleton(handle.clone(), GitReposStore::new(handle.clone())),
            singleton(handle.clone(), ChordPackageManager::new(handle.clone())),
            singleton(handle.clone(), ChordActionTaskRunner::new(handle.clone())),
            singleton(handle.clone(), DesktopAppManager::new(handle.clone())),
        );

        s.0.init(manage(app.handle(), FrontmostObservable::new(handle.clone())?))?;
        s.1.init(manage(app.handle(), ChorderObservable::new(handle.clone())?))?;
        // s.2.init()?;
        // s.3.init()?;
        s.4.init(manage(
            app.handle(),
            AppPermissionsObservable::new(handle.clone())?,
        ))?;
        s.5.init(manage(
            app.handle(),
            AppSettingsObservable::new(handle.clone())?,
        ))?;
        // s.6.init()?;
        // s.7.init()?;
        s.8.init(manage(app.handle(), GitReposObservable::new(handle.clone())?))?;
        s.9.init(manage(
            app.handle(),
            ChordPackageManagerObservable::new(handle.clone())?,
        ))?;
        // s.10.init()?;
        s.11.init(manage(
            app.handle(),
            DesktopAppManagerObservable::new(handle.clone())?,
        ))?;
    };

    let handle = app.handle();
    tauri_app::scripting::init(handle.clone());
    tauri::async_runtime::spawn(load_chord_packages(handle.clone()));
    tauri::async_runtime::spawn(load_permissions(handle.clone()));

    // Create tray
    if let Err(error) = tauri_app::tray::create_tray(handle.clone()) {
        log::error!("Failed to create tray: {error:#}");
    }
    let settings = handle.app_settings();
    settings.apply_all()?;

    let startup_status = tauri_app::startup::get_startup_status(&handle)?;
    if startup_status.should_show_onboarding {
        settings.ui.open()?;
    }

    Ok(())
}

async fn load_chord_packages(handle: AppHandle) -> anyhow::Result<()> {
    let chord_pm = handle.chord_package_manager();
    chord_pm.reload_all().await?;
    Ok(())
}

async fn load_permissions(handle: AppHandle) -> anyhow::Result<()> {
    let permissions = handle.app_permissions();
    Ok(permissions.load().await?)
}

fn manage<T>(handle: &AppHandle, value: T) -> T
where
    T: Send + Sync + Clone + 'static,
{
    handle.manage(value.clone());
    value
}

unsafe fn singleton<T>(handle: AppHandle, value: T) -> &'static mut T
where
    T: Send + Sync + 'static,
{
    // 1. Box the value to get a stable heap address
    let mut boxed = Box::new(value);

    // 2. Get a raw pointer to the data inside the box
    let ptr: *mut T = &mut *boxed;

    // 3. Move the box (and the value T) into Tauri.
    // Since it's a Box, the data at `ptr` does NOT move in memory.
    handle.manage(*boxed);

    // 4. Return the pointer as a static mutable reference.
    // SAFETY: This works because 'handle.manage' now owns the heap allocation.
    // The memory will live as long as the AppHandle/State exists.
    &mut *ptr
}