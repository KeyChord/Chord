use crate::app::AppHandleExt;
use crate::app::chord_package_manager::ChordPackageManager;
use crate::app::chord_package_registry::ChordPackageRegistry;
use crate::app::chord_package_store::ChordPackageStore;
use crate::app::chord_runner::ChordActionTaskRunner;
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
use crate::app::state::StateSingleton;
use crate::chordpack::load_default_chordpack;
use crate::lock_file::AppLockFile;
use crate::observables::{
    AppPermissionsObservable, AppSettingsObservable, ChordPackageManagerObservable,
    ChordPackageStoreObservable, ChorderObservable, DesktopAppManagerObservable,
    FrontmostObservable, GitReposObservable, Observable,
};
use crate::tauri_app;
use tauri::{AppHandle, Manager};

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
pub fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    let handle = app.handle().clone();
    let app_lock_file = AppLockFile::acquire(app.handle())?;
    app.handle().manage(app_lock_file);

    let s = (
        singleton(&handle, AppFrontmost::new(handle.clone())),
        singleton(&handle, AppChorder::new(handle.clone())),
        singleton(&handle, AppContext::new(handle.clone())),
        singleton(&handle, DevLockfileDetector::new()),
        singleton(&handle, AppPermissions::new(handle.clone())),
        singleton(&handle, AppSettings::new(handle.clone())),
        singleton(&handle, GlobalHotkeyStore::new(handle.clone())),
        singleton(&handle, PlaceholderChordStore::new(handle.clone())),
        singleton(&handle, GitReposStore::new(handle.clone())),
        singleton(&handle, ChordPackageManager::new(handle.clone())),
        singleton(&handle, ChordActionTaskRunner::new(handle.clone())),
        singleton(&handle, DesktopAppManager::new(handle.clone())),
        singleton(&handle, ChordPackageStore::new(handle.clone())),
        singleton(&handle, ChordPackageRegistry::new(handle.clone())),
    );

    s.0.init(manage(
        app.handle(),
        FrontmostObservable::new(handle.clone())?,
    ))?;
    s.1.init(manage(
        app.handle(),
        ChorderObservable::new(handle.clone())?,
    ))?;
    s.2.init()?;
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
    s.8.init(manage(
        app.handle(),
        GitReposObservable::new(handle.clone())?,
    ))?;
    s.9.init(manage(
        app.handle(),
        ChordPackageManagerObservable::new(handle.clone())?,
    ))?;
    // s.10.init()?;
    s.11.init(manage(
        app.handle(),
        DesktopAppManagerObservable::new(handle.clone())?,
    ))?;
    s.12.init(manage(
        app.handle(),
        ChordPackageStoreObservable::new(handle.clone())?,
    ))?;
    // s.13.init()?;

    log::debug!("initialized all singletons");

    let handle = app.handle();
    tauri_app::scripting::init(handle.clone());

    log::info!("Loading permissions synchronously to register input handlers immediately");
    if let Err(e) = tauri::async_runtime::block_on(handle.app_permissions().load()) {
        log::error!("Failed to load permissions: {e}");
    }

    tauri::async_runtime::spawn(load_chord_packages(handle.clone()));

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

    log::debug!("finished setup()");

    Ok(())
}

async fn load_chord_packages(handle: AppHandle) -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    {
        log::debug!("Development mode detected, syncing default chordpack");
        let store = handle.app_git_repos_store();
        let default_chordpack = load_default_chordpack()?;
        store.ensure_pinned_repos(default_chordpack)?;
    }

    let chord_pm = handle.chord_package_manager();
    chord_pm.reload_all().await?;
    Ok(())
}

fn manage<T>(handle: &AppHandle, value: T) -> T
where
    T: Send + Sync + Clone + 'static,
{
    handle.manage(value.clone());
    value
}

fn singleton<T>(handle: &AppHandle, value: T) -> tauri::State<'_, T>
where
    T: Send + Sync + 'static,
{
    handle.manage(value);
    let value = handle.state::<T>();
    value
}
