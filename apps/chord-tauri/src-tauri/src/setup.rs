use crate::app::chord_mode_manager::{ChordModeManager, ChordModeManagerProvider};
use crate::app::chord_package_manager::{ChordPackageManager, ChordPackageManagerProvider};
use crate::app::chord_package_store::{ChordPackageStore, ChordPackageStoreProvider};
use crate::app::chord_runner::{ChordActionTaskRunner, ChordActionTaskRunnerProvider};
use crate::app::controller::{AppController, AppControllerProvider};
use crate::app::desktop_app::{DesktopAppManager, DesktopAppManagerProvider};
use crate::app::dev_lockfile_detector::{DevLockfileDetector, DevLockfileDetectorProvider};
use crate::app::frontmost::{AppFrontmost, AppFrontmostProvider};
use crate::app::global_hotkey_store::{GlobalHotkeyStore, GlobalHotkeyStoreProvider};
use crate::app::keyboard::{AppKeyboard, AppKeyboardProvider};
use crate::app::permissions::{AppPermissions, AppPermissionsProvider};
use crate::app::placeholder_chord_store::{PlaceholderChordStore, PlaceholderChordStoreProvider};
use crate::app::settings::{AppSettings, AppSettingsProvider};
use crate::app::{AppHandleExt, AppSingleton};
use crate::chordpack::load_default_chordpack;
use crate::lock_file::AppLockFile;
use crate::state::{
    AppModeObservable, AppPermissionsObservable, AppSettingsObservable, ChordInputObservable,
    ChordPackageManagerObservable, ChordPackageStoreObservable, ChordPanelObservable,
    DesktopAppManagerObservable, FrontmostObservable, GitRepo, GitReposObservable,
    KeyboardObservable, Observable,
};
use crate::tauri_app;
use anyhow::Result;
use tauri::AppHandle;

pub fn setup(app: &mut tauri::App) -> Result<()> {
    let handle = app.handle().clone();
    let app_lock_file = AppLockFile::acquire(app.handle())?;
    {
        use tauri::Manager;
        app.handle().manage(app_lock_file);
    }

    let app_mode_observable = AppModeObservable::new(app)?;
    let app_permissions_observable = AppPermissionsObservable::new(app)?;
    let app_settings_observable = AppSettingsObservable::new(app)?;
    let chord_input_observable = ChordInputObservable::new(app)?;
    let chord_panel_observable = ChordPanelObservable::new(app)?;
    let chord_package_store_observable = ChordPackageStoreObservable::new(app)?;
    let desktop_app_manager_observable = DesktopAppManagerObservable::new(app)?;
    let chord_package_manager_observable = ChordPackageManagerObservable::new(app)?;
    let git_repos_observable = GitReposObservable::new(app)?;
    let frontmost_observable = FrontmostObservable::new(app)?;
    let keyboard_observable = KeyboardObservable::new(app)?;

    let managed = Managed {
        handle: handle.clone(),
        init_fns: Vec::new(),
    };

    managed
        .add(
            ChordModeManagerProvider {
                handle: handle.clone(),
                chord_input_observable,
                chord_panel_observable,
            }
            .provide::<ChordModeManager>(),
        )
        .add(
            ChordPackageManagerProvider {
                handle: handle.clone(),
                chord_package_manager_observable,
                git_repos_observable,
            }
            .provide::<ChordPackageManager>(),
        )
        .add(
            ChordPackageStoreProvider {
                handle: handle.clone(),
                chord_package_store_observable,
            }
            .provide::<ChordPackageStore>(),
        )
        .add(
            ChordActionTaskRunnerProvider {
                handle: handle.clone(),
            }
            .provide::<ChordActionTaskRunner>(),
        )
        .add(
            DesktopAppManagerProvider {
                handle: handle.clone(),
                desktop_app_manager_observable,
            }
            .provide::<DesktopAppManager>(),
        )
        .add(DevLockfileDetectorProvider.provide::<DevLockfileDetector>())
        .add(
            AppFrontmostProvider {
                handle: handle.clone(),
                frontmost_observable,
            }
            .provide::<AppFrontmost>(),
        )
        .add(
            GlobalHotkeyStoreProvider {
                handle: handle.clone(),
            }
            .provide::<GlobalHotkeyStore>(),
        )
        .add(
            AppKeyboardProvider {
                handle: handle.clone(),
                keyboard_observable,
            }
            .provide::<AppKeyboard>(),
        )
        .add(
            AppControllerProvider {
                handle: handle.clone(),
                app_mode_observable,
            }
            .provide::<AppController>(),
        )
        .add(
            AppPermissionsProvider {
                handle: handle.clone(),
                app_permissions_observable,
            }
            .provide::<AppPermissions>(),
        )
        .add(
            PlaceholderChordStoreProvider {
                handle: handle.clone(),
            }
            .provide::<PlaceholderChordStore>(),
        )
        .add(
            AppSettingsProvider {
                handle: handle.clone(),
                app_settings_observable,
            }
            .provide::<AppSettings>(),
        )
        .init()?;

    tauri_app::scripting::init(handle.clone());

    log::info!("Loading permissions synchronously to register input handlers immediately");
    let state = handle.app_state();
    if let Err(e) = tauri::async_runtime::block_on(state.permissions().load()) {
        log::error!("Failed to load permissions: {e}");
    }

    log::info!("Pre-warming chorder UI");
    if let Err(e) = state.chord_mode_manager().panel.preload() {
        log::error!("Failed to preload chorder UI: {e}");
    }

    let packages_handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = load_chord_packages(packages_handle.clone()).await {
            log::error!("Failed to load chord packages: {error:#}");
        }
        tauri_app::scripting::mark_chord_packages_ready(&packages_handle);
    });

    // Create tray
    if let Err(error) = tauri_app::tray::create_tray(handle.clone()) {
        log::error!("Failed to create tray: {error:#}");
    }
    let settings = state.settings();
    settings.apply_all()?;

    if tauri_app::startup::should_show_permission_dialog() {
        settings.ui.open()?;
    }

    log::debug!("finished setup()");

    Ok(())
}

async fn load_chord_packages(handle: AppHandle) -> anyhow::Result<()> {
    let state = handle.app_state();
    let store = &state.chord_package_manager().registry.git.git_repos_store;
    let is_first_run = !store.has_persisted_state()?;

    if cfg!(debug_assertions) || is_first_run {
        log::debug!(
            "Syncing default chordpack (development: {}, first run: {})",
            cfg!(debug_assertions),
            is_first_run
        );
        let sync_result = load_default_chordpack()
            .and_then(|default_chordpack| store.ensure_pinned_repos(default_chordpack));
        if let Err(error) = sync_result {
            log::error!(
                "Failed to sync the default chordpack; loading the existing active repositories: {error:#}"
            );
        }
    }

    let chord_pm = state.chord_package_manager();
    chord_pm.reload_all().await?;
    Ok(())
}

struct Managed {
    pub handle: AppHandle,
    init_fns: Vec<Box<dyn FnOnce() -> Result<()>>>,
}

impl Managed {
    fn add<T: AppSingleton>(mut self, value: T) -> Self {
        use tauri::Manager;
        self.handle.manage(value);

        let handle = self.handle.clone();
        self.init_fns.push(Box::new(move || {
            let state = Manager::state::<T>(&handle);
            state.init()
        }));

        self
    }

    fn init(self) -> Result<()> {
        for init_fn in self.init_fns {
            init_fn()?;
        }
        Ok(())
    }
}
