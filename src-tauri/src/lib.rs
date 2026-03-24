use crate::api::{Api, ApiImpl};
use crate::observables::{ChordFilesObservable, FrontmostObservable, Observable};
use crate::tauri_app::registry::ChordPackageRegistry;
use anyhow::Result;
use parking_lot::deadlock;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, RunEvent};
pub use tauri_app::*;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::{Target, TargetKind};

mod api;
mod chords;
mod constants;
mod feature;
pub mod git;
mod input;
mod mode;
mod observables;
mod tauri_app;

use crate::chords::{ChordPackage, ChordRegistry};
use crate::feature::app_handle_ext::{AppHandleExt, AppManaged};
use crate::feature::global_hotkey::GlobalHotkeyStore;
use crate::feature::placeholder_chords::PlaceholderChordStore;
use crate::feature::repos::GitReposStore;
use crate::feature::{AppChorder, AppFrontmost, AppPermissions, AppSettings, SafeAppHandle};
use crate::observables::{
    AppPermissionsObservable, AppSettingsObservable, ChorderObservable, GitReposObservable,
};
use crate::registry::GitPackageRegistry;
use tauri_nspanel::tauri_panel;

tauri_panel! {
    panel!(IndicatorPanel {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
            accepts_first_responder: false,
            becomes_key_only_if_needed: false,
            is_floating_panel: true,
            hides_on_deactivate: false
        }
    })
}

#[cfg_attr(mobile, tauri_app::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();

        eprintln!("PANIC: {info}");
        eprintln!("{bt}");

        log::error!("PANIC: {info}");
        log::error!("{bt}");
    }));

    // https://github.com/Narsil/rdev/issues/165#issuecomment-2907684547
    #[cfg(target_os = "macos")]
    rdev::set_is_main_thread(false);

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                continue;
            }

            log::warn!("{} deadlocks detected", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                log::warn!("Deadlock #{}", i);
                for t in threads {
                    log::warn!("Thread Id {:#?}", t.thread_id());
                    log::warn!("{:#?}", t.backtrace());
                }
            }
        }
    });

    let log_plugin = tauri_plugin_log::Builder::new()
        .clear_targets()
        .level(log::LevelFilter::Debug)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("chords".into()),
            }),
            Target::new(TargetKind::Webview),
        ])
        .build();

    let api_impl = ApiImpl::default();
    let api_impl_for_setup = api_impl.clone();

    let app = tauri::Builder::default()
        .invoke_handler(taurpc::create_ipc_handler(api_impl.into_handler()))
        .menu(|handle| tauri_app::menu::build_app_menu(handle))
        .on_menu_event(|handle, event| {
            tauri_app::menu::handle_menu_event(handle, &event);
        })
        .plugin(log_plugin)
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|handle, _args, _cwd| {
            if let Err(error) = tauri_app::settings::show_settings_window(handle.clone()) {
                log::error!("Failed to show settings window for existing instance: {error}");
            }
        }))
        .plugin(tauri_nspanel::init())
        .plugin({
            #[cfg(target_os = "macos")]
            {
                tauri_plugin_autostart::Builder::new()
                    .macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent)
                    .args(["--autostart"])
                    .build()
            }
            #[cfg(not(target_os = "macos"))]
            {
                tauri_plugin_autostart::Builder::new()
                    .args(["--autostart"])
                    .build()
            }
        })
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_user_input::init())
        .setup(move |app| {
            api_impl_for_setup.set_app_handle(app.handle().clone());
            if let Err(e) = setup(app) {
                log::error!("Failed to set up app:\n{:#?}", e);
                app.dialog()
                    .message(format!("Failed to start Chord:\n\n{e}"))
                    .title("Startup Error")
                    .blocking_show();

                std::process::exit(1);
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri_app application");

    app.run(|handle, event| {
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = event {
            if let Err(error) = tauri_app::settings::show_settings_window(handle.clone()) {
                log::error!("Failed to show settings window after dock reopen: {error}");
            }
        }
    });
}

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
fn setup(app: &mut tauri::App) -> Result<()> {
    let safe_handle = SafeAppHandle::new(app.handle().clone())?;
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
) -> Result<()> {
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

async fn load_permissions(handle: AppHandle) -> Result<()> {
    let permissions = handle.app_permissions();
    Ok(permissions.load().await?)
}
