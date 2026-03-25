use std::sync::Arc;
use tauri::{AppHandle, Manager};
use crate::{tauri_app, AppContext};
use crate::chords::{ChordPackage, ChordRegistry};
use crate::app::{AppHandleExt, AppManaged};
use crate::app::{AppChorder, AppFrontmost, AppPermissions, AppSettings, SafeAppHandle};
use crate::app::global_hotkey::GlobalHotkeyStore;
use crate::app::placeholder_chords::PlaceholderChordStore;
use crate::app::repos::GitReposStore;
use crate::lock_file::AppLockFile;
use crate::observables::{AppPermissionsObservable, AppSettingsObservable, ChordFilesObservable, ChorderObservable, FrontmostObservable, GitReposObservable};
use crate::registry::{ChordPackageRegistry, GitPackageRegistry};

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
pub fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    let safe_handle = SafeAppHandle::new(app.handle().clone())?;
    let app_lock_file = AppLockFile::acquire(app.handle())?;
    let chorder_observable = Arc::new(ChorderObservable::new(safe_handle.clone())?);
    let git_repos_observable = Arc::new(GitReposObservable::new(safe_handle.clone())?);
    let permissions_observable = Arc::new(AppPermissionsObservable::new(safe_handle.clone())?);
    let settings_observable = Arc::new(AppSettingsObservable::new(safe_handle.clone())?);
    let frontmost_observable = Arc::new(FrontmostObservable::new(safe_handle.clone())?);
    let chord_files_observable = Arc::new(ChordFilesObservable::new(safe_handle.clone())?);
    let git_package_registry = Arc::new(GitPackageRegistry::new(safe_handle.clone())?);
    app.handle().manage(chorder_observable.clone());
    app.handle().manage(git_repos_observable.clone());
    app.handle().manage(permissions_observable.clone());
    app.handle().manage(settings_observable.clone());
    app.handle().manage(frontmost_observable.clone());
    app.handle().manage(chord_files_observable.clone());
    app.handle().manage(git_package_registry.clone());
    app.handle().manage(app_lock_file);
    safe_handle.manage(AppManaged {
        frontmost: AppFrontmost::new_with_detector(frontmost_observable.clone())?,
        chorder: AppChorder::new(safe_handle.clone(), chorder_observable.clone())?,
        context: AppContext::new()?,
        permissions: AppPermissions::new_unloaded(
            safe_handle.clone(),
            permissions_observable.clone(),
        )?,
        settings: AppSettings::new(safe_handle.clone(), settings_observable.clone())?,
        chord_package_registry: ChordPackageRegistry::new_unloaded(safe_handle.clone())?,
        global_hotkey_store: GlobalHotkeyStore::new(safe_handle.clone())?,
        placeholder_chord_store: PlaceholderChordStore::new(safe_handle.clone())?,
        git_repos_store: GitReposStore::new(safe_handle.clone(), git_repos_observable.clone())?,
        chord_registry: ChordRegistry::new_empty(
            safe_handle.clone(),
            chord_files_observable.clone(),
        ),
    });

    let handle = app.handle();
    tauri_app::scripting::init(handle.clone());
    tauri::async_runtime::spawn(load_chords(handle.clone(), git_package_registry));
    tauri::async_runtime::spawn(load_permissions(handle.clone()));

    // Create tray
    if let Err(error) = tauri_app::tray::create_tray(handle.clone()) {
        log::error!("Failed to create tray: {error:#}");
    }
    handle.app_settings().apply_all()?;

    let startup_status = tauri_app::startup::get_startup_status(&handle)?;
    if startup_status.should_show_onboarding {
        tauri_app::settings::show_settings_window(handle.clone())?;
    }

    Ok(())
}

async fn load_chords(
    handle: AppHandle,
    git_package_registry: Arc<GitPackageRegistry>,
) -> anyhow::Result<()> {
    let mut chord_packages = git_package_registry.load_all_packages()?;
    chord_packages.push(ChordPackage::load_bundled()?);
    let chord_registry = handle.app_chord_registry();
    log::debug!(
        "Loading packages: {:?}",
        chord_packages
            .iter()
            .map(|p| p.root_dir.clone())
            .collect::<Vec<_>>()
    );
    chord_registry.load_packages(chord_packages).await?;
    Ok(())
}

async fn load_permissions(handle: AppHandle) -> anyhow::Result<()> {
    let permissions = handle.app_permissions();
    Ok(permissions.load().await?)
}
