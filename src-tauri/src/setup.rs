use crate::app::{AppHandleExt, AppSingleton};
use crate::app::chord_package_manager::ChordPackageManager;
use crate::app::chord_package_registry::ChordPackageRegistry;
use crate::app::chord_package_store::ChordPackageStore;
use crate::app::chord_runner::ChordActionTaskRunner;
use crate::app::desktop_app::DesktopAppManager;
use crate::app::dev_lockfile_detector::DevLockfileDetector;
use crate::app::frontmost::AppFrontmost;
use crate::app::git_repos_store::GitReposStore;
use crate::app::global_hotkey_store::GlobalHotkeyStore;
use crate::app::permissions::AppPermissions;
use crate::app::placeholder_chord_store::PlaceholderChordStore;
use crate::app::settings::AppSettings;
use crate::chordpack::load_default_chordpack;
use crate::lock_file::AppLockFile;
use crate::state::{AppModeObservable, AppPermissionsObservable, AppSettingsObservable, ChordPackageManagerObservable, ChordPackageStoreObservable, DesktopAppManagerObservable, FrontmostObservable, GitReposObservable, Observable};
use crate::tauri_app;
use tauri::{AppHandle, Manager};
use crate::app::mode::AppModeManager;
use anyhow::Result;

struct SingletonSetup {
    callbacks: Vec<Box<dyn FnOnce() -> Result<()>>>,
    handle: AppHandle
}

impl SingletonSetup {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle, callbacks: Vec::new() }
    }

    fn manage<N, I: 'static, S: AppSingleton<I>>(mut self, new: N, init: I) -> Self
    where
        N: FnOnce(AppHandle) -> S
    {
        let handle = self.handle.clone();
        let singleton = new(handle.clone());
        handle.manage(singleton);
        self.callbacks.push(Box::new(move || {
            let value = handle.state::<S>();
            value.init(init)
        }));
        self
    }

    fn init(self) -> Result<()> {
        for callback in self.callbacks {
            callback()?;
        }
        Ok(())
    }
}


// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
pub fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    let handle = app.handle().clone();
    let app_lock_file = AppLockFile::acquire(app.handle())?;
    app.handle().manage(app_lock_file);

    SingletonSetup::new(handle.clone())
        .manage(AppFrontmost::new, FrontmostObservable::new(handle.clone())?)
        .manage(AppModeManager::new, AppModeObservable::new(handle.clone())?)
        .manage(DevLockfileDetector::new, ())
        .manage(AppPermissions::new, AppPermissionsObservable::new(handle.clone())?)
        .manage(AppSettings::new, AppSettingsObservable::new(handle.clone())?)
        .manage(GlobalHotkeyStore::new, ())
        .manage(PlaceholderChordStore::new, ())
        .manage(GitReposStore::new, ())
        .manage(ChordPackageManager::new, ChordPackageManagerObservable::new(handle.clone())?)
        .manage(ChordActionTaskRunner::new, ())
        .manage(DesktopAppManager::new, DesktopAppManagerObservable::new(handle.clone())?)
        .manage(ChordPackageStore::new, ChordPackageStoreObservable::new(handle.clone())?)
        .manage(ChordPackageRegistry::new, ())
        .init()?;

    tauri_app::scripting::init(handle.clone());

    log::info!("Loading permissions synchronously to register input handlers immediately");
    let state = AppHandleExt::state(app.handle());
    if let Err(e) = tauri::async_runtime::block_on(state.permissions().load()) {
        log::error!("Failed to load permissions: {e}");
    }

    log::info!("Pre-warming chorder UI");
    if let Err(e) = handle.app_chord_mode().preload_ui() {
        log::error!("Failed to preload chorder UI: {e}");
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
