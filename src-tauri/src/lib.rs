use crate::api::{Api, ApiImpl};
use crate::chords::{ChordPackage, LoadedAppChords};
use crate::feature::{AppSettings, AppChorder, ChorderIndicatorUi, ChorderState, AppSettingsState, AppPermissions, AppFrontmost, SettingsUi, AppSettingsStateGitRepo, SafeAppHandle};
use crate::input::{register_caps_lock_input_handler, register_key_event_input_grabber};
use anyhow::Result;
use frontmost::{start_nsrunloop, Detector};
use objc2_app_kit::NSWorkspace;
use parking_lot::deadlock;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};
pub use tauri_app::*;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::{Target, TargetKind};

mod api;
mod chords;
mod constants;
mod feature;
mod git;
mod input;
mod mode;
mod sources;
mod tauri_app;

use tauri_nspanel::tauri_panel;
use tauri_plugin_autostart::ManagerExt;
use crate::tauri_app::git::ChordPackageRegistry;

tauri_panel! {
    panel!(IndicatorPanel {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
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

    thread::spawn(move || loop {
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

    tauri::Builder::default()
        .invoke_handler(taurpc::create_ipc_handler(api_impl.into_handler()))
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
        .run(tauri::generate_context!())
        .expect("error while running tauri_app application");
}

async fn create_app_settings(
    safe_handle: SafeAppHandle,
) -> Result<AppSettings, anyhow::Error> {
    let chord_package_registry = ChordPackageRegistry::new(
        safe_handle,
    );

    let settings_state = AppSettingsState {
        bundle_ids_needing_relaunch: Vec::new(),
        git_repos: chord_package_registry
            .git
            .discover_git_repos()?
            .iter()
            .map(|r| AppSettingsStateGitRepo {
                owner: r.owner.clone(),
                name: r.name.clone(),
                slug: r.slug.clone(),
                url: r.url.clone(),
                local_path: r.local_path.clone(),
                head_short_sha: r.head_short_sha.clone(),
            })
            .collect(),
        permissions: AppPermissions::from_check(safe_handle.clone()).await,
    };

    let settings = AppSettings::new(safe_handle.clone(), settings_state);
    Ok(settings)
}

async fn create_app_permissions(safe_handle: SafeAppHandle) -> Result<AppPermissions> {
    AppPermissions::from_check(safe_handle).await
}

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
fn setup(app: &mut tauri::App) -> Result<()> {
    let safe_handle = SafeAppHandle::new(app.handle().clone());

    let app_permissions = tauri::async_runtime::block_on(create_app_permissions(safe_handle))?;
    app.manage(app_permissions);

    let app_frontmost = AppFrontmost::new_with_detector();
    app.manage(app_frontmost);

    let chorder_state = ChorderState::default();
    let chorder = AppChorder::new(app.handle().clone(), chorder_state)?;
    app.manage(chorder);

    let context = AppContext::new()?;
    app.manage(context);

    let app_settings = tauri::async_runtime::block_on(create_app_settings(app))?;
    app.manage(app_settings);


    let handle = app.handle();
    let tray_created = match tauri_app::tray::create_tray(handle.clone()) {
        Ok(()) => true,
        Err(error) => {
            log::error!("Failed to create tray: {error:#}");
            false
        }
    };

    {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = reload_loaded_app_chords(handle).await {
                log::error!("Failed to reload app chords:\n{:#?}", e);
            }
        });
    }

    Ok(())
}
