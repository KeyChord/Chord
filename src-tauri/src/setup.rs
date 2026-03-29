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
use crate::app::{AppHandleExt, AppManaged, SafeAppHandle};
use crate::lock_file::AppLockFile;
use crate::observables::{AppPermissionsObservable, AppSettingsObservable, ChordPackageManagerObservable, ChorderObservable, DesktopAppManagerObservable, FrontmostObservable, GitReposObservable, Observable};
use crate::tauri_app;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use crate::app::chord_package_manager::ChordPackageManager;

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
pub fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    let safe_handle = SafeAppHandle::new(app.handle().clone())?;

    let app_lock_file = AppLockFile::acquire(app.handle())?;
    app.handle().manage(app_lock_file);

    let chorder_observable = manage(app.handle(), ChorderObservable::new(safe_handle.clone())?);
    let git_repos_observable = manage(app.handle(), GitReposObservable::new(safe_handle.clone())?);
    let permissions_observable = manage(
        app.handle(),
        AppPermissionsObservable::new(safe_handle.clone())?,
    );
    let settings_observable = manage(
        app.handle(),
        AppSettingsObservable::new(safe_handle.clone())?,
    );
    let frontmost_observable = manage(app.handle(), FrontmostObservable::new(safe_handle.clone())?);
    let chord_package_manager_observable = manage(
        app.handle(),
        ChordPackageManagerObservable::new(safe_handle.clone())?,
    );
    let desktop_app_manager_observable = manage(
        app.handle(),
        DesktopAppManagerObservable::new(safe_handle.clone())?,
    );

    safe_handle.manage(AppManaged {
        frontmost: AppFrontmost::new_with_detector(frontmost_observable.clone())?,
        chorder: AppChorder::new(safe_handle.clone(), chorder_observable.clone())?,
        context: AppContext::new()?,
        dev_lockfile_detector: DevLockfileDetector::new(),
        permissions: AppPermissions::new_unloaded(
            safe_handle.clone(),
            permissions_observable.clone(),
        )?,
        settings: AppSettings::new(safe_handle.clone(), settings_observable.clone())?,
        global_hotkey_store: GlobalHotkeyStore::new(safe_handle.clone())?,
        placeholder_chord_store: PlaceholderChordStore::new(safe_handle.clone())?,
        git_repos_store: GitReposStore::new(safe_handle.clone(), git_repos_observable.clone())?,
        chord_package_manager: ChordPackageManager::new(safe_handle.clone(), chord_package_manager_observable)?,
        chord_action_task_runner: ChordActionTaskRunner::new(safe_handle.clone()),
        desktop_app_manager: DesktopAppManager::new(
            safe_handle.clone(),
            desktop_app_manager_observable,
        ),
    });

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

fn manage<T>(handle: &tauri::AppHandle, value: T) -> Arc<T>
where
    T: Send + Sync + 'static,
{
    let value = Arc::new(value);
    handle.manage(value.clone());
    value
}
