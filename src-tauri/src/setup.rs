use crate::app::chord_mode_manager::{ChordModeManager, ChordModeManagerProvider};
use crate::app::chord_package_manager::{ChordPackageManager, ChordPackageManagerProvider};
use crate::app::chord_package_store::{ChordPackageStore, ChordPackageStoreProvider};
use crate::app::chord_runner::{ChordActionTaskRunner, ChordActionTaskRunnerProvider};
use crate::app::desktop_app::{DesktopAppManager, DesktopAppManagerProvider};
use crate::app::dev_lockfile_detector::{DevLockfileDetector, DevLockfileDetectorProvider};
use crate::app::frontmost::{AppFrontmost, AppFrontmostProvider};
use crate::app::global_hotkey_store::{GlobalHotkeyStore, GlobalHotkeyStoreProvider};
use crate::app::keyboard::{AppKeyboard, AppKeyboardProvider};
use crate::app::mode::{AppModeManager, AppModeManagerProvider};
use crate::app::permissions::{AppPermissions, AppPermissionsProvider};
use crate::app::placeholder_chord_store::{PlaceholderChordStore, PlaceholderChordStoreProvider};
use crate::app::settings::{AppSettings, AppSettingsProvider};
use crate::app::{AppHandleExt, AppSingleton};
use crate::chordpack::load_default_chordpack;
use crate::lock_file::AppLockFile;
use crate::state::{
    AppModeObservable, AppPermissionsObservable, AppSettingsObservable, ChordInputObservable,
    ChordModeObservable, ChordPackageManagerObservable, ChordPackageStoreObservable,
    DesktopAppManagerObservable, FrontmostObservable, GitRepo, GitReposObservable, Observable,
};
use crate::tauri_app;
use anyhow::Result;
use tauri::{AppHandle, Manager};

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
pub fn setup(app: &mut tauri::App) -> Result<()> {
    let handle = app.handle().clone();
    let app_lock_file = AppLockFile::acquire(app.handle())?;
    app.handle().manage(app_lock_file);

    let app_mode_observable = AppModeObservable::new(handle.clone())?;
    let app_permissions_observable = AppPermissionsObservable::new(handle.clone())?;
    let app_settings_observable = AppSettingsObservable::new(handle.clone())?;
    let chord_input_observable = ChordInputObservable::new(handle.clone())?;
    let chord_mode_observable = ChordModeObservable::new(handle.clone())?;
    let desktop_app_manager_observable = DesktopAppManagerObservable::new(handle.clone())?;
    let chord_package_manager_observable = ChordPackageManagerObservable::new(handle.clone())?;
    let git_repos_observable = GitReposObservable::new(handle.clone())?;
    let frontmost_observable = FrontmostObservable::new(handle.clone())?;

    let mut app_state: Vec<Box<dyn AppSingleton>> = vec![
        Box::new(
            ChordModeManagerProvider {
                handle: handle.clone(),
                chord_input_observable,
                chord_mode_observable,
            }
            .provide::<ChordModeManager>(),
        ),
        Box::new(
            ChordPackageManagerProvider {
                handle: handle.clone(),
                chord_package_manager_observable,
                git_repos_observable,
            }
            .provide::<ChordPackageManager>(),
        ),
        Box::new(
            ChordPackageStoreProvider {
                handle: handle.clone(),
            }
            .provide::<ChordPackageStore>(),
        ),
        Box::new(
            ChordActionTaskRunnerProvider {
                handle: handle.clone(),
            }
            .provide::<ChordActionTaskRunner>(),
        ),
        Box::new(
            DesktopAppManagerProvider {
                handle: handle.clone(),
                desktop_app_manager_observable,
            }
            .provide::<DesktopAppManager>(),
        ),
        Box::new(DevLockfileDetectorProvider.provide::<DevLockfileDetector>()),
        Box::new(
            AppFrontmostProvider {
                handle: handle.clone(),
                frontmost_observable,
            }
            .provide::<AppFrontmost>(),
        ),
        Box::new(
            GlobalHotkeyStoreProvider {
                handle: handle.clone(),
            }
            .provide::<GlobalHotkeyStore>(),
        ),
        Box::new(
            AppKeyboardProvider {
                handle: handle.clone(),
            }
            .provide::<AppKeyboard>(),
        ),
        Box::new(
            AppModeManagerProvider {
                handle: handle.clone(),
                app_mode_observable,
            }
            .provide::<AppModeManager>(),
        ),
        Box::new(
            AppPermissionsProvider {
                handle: handle.clone(),
                app_permissions_observable,
            }
            .provide::<AppPermissions>(),
        ),
        Box::new(
            PlaceholderChordStoreProvider {
                handle: handle.clone(),
            }
            .provide::<PlaceholderChordStore>(),
        ),
        Box::new(
            AppSettingsProvider {
                handle: handle.clone(),
                app_settings_observable,
            }
            .provide::<AppSettings>(),
        ),
    ];

    tauri_app::scripting::init(handle.clone());

    log::info!("Loading permissions synchronously to register input handlers immediately");
    let state = AppHandleExt::state(app.handle());
    if let Err(e) = tauri::async_runtime::block_on(state.permissions().load()) {
        log::error!("Failed to load permissions: {e}");
    }

    log::info!("Pre-warming chorder UI");
    if let Err(e) = state.chord_mode_manager().ui.preload() {
        log::error!("Failed to preload chorder UI: {e}");
    }

    tauri::async_runtime::spawn(load_chord_packages(handle.clone()));

    // Create tray
    if let Err(error) = tauri_app::tray::create_tray(handle.clone()) {
        log::error!("Failed to create tray: {error:#}");
    }
    let settings = state.settings();
    settings.apply_all()?;

    let startup_status = tauri_app::startup::get_startup_status(&handle)?;
    if startup_status.should_show_onboarding {
        settings.ui.open()?;
    }

    log::debug!("finished setup()");

    Ok(())
}

async fn load_chord_packages(handle: AppHandle) -> anyhow::Result<()> {
    let state = AppHandleExt::state(&handle);

    #[cfg(debug_assertions)]
    {
        log::debug!("Development mode detected, syncing default chordpack");
        let store = state.chord_package_manager().registry.git.git_repos_store;
        let default_chordpack = load_default_chordpack()?;
        store.ensure_pinned_repos(default_chordpack)?;
    }

    let chord_pm = state.chord_package_manager();
    chord_pm.reload_all().await?;
    Ok(())
}
