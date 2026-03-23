use crate::api::{Api, ApiImpl};
use crate::tauri_app::git::ChordPackageRegistry;
use anyhow::Result;
use parking_lot::deadlock;
use std::thread;
use std::time::Duration;
use tauri::Manager;
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
mod tauri_app;
mod observables;

use tauri_nspanel::tauri_panel;
use crate::feature::app_handle_ext::AppManaged;
use crate::feature::{AppChorder, AppFrontmost, AppPermissions, AppSettings, SafeAppHandle};
use crate::observables::{AppSettingsState, ChorderState};

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

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
fn setup(app: &mut tauri::App) -> Result<()> {
    let safe_handle = SafeAppHandle::new(app.handle().clone());
    safe_handle.mark_safe(AppManaged {
        frontmost: AppFrontmost::new_with_detector(),
        chorder: AppChorder::new(safe_handle.clone())?,
        context: AppContext::new()?,
        permissions: AppPermissions::new(safe_handle.clone()),
        settings: AppSettings::new(safe_handle.clone())?,
        chord_package_registry: ChordPackageRegistry::new(safe_handle.clone())?
    });

    let handle = app.handle();
    if let Err(error) = tauri_app::tray::create_tray(handle.clone()) {
        log::error!("Failed to create tray: {error:#}");
    }

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
