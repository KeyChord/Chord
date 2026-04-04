use crate::api::{Api, ApiImpl};
use crate::app::AppHandleExt;
use crate::setup::setup;
use crate::tauri_app::lock_file::AppLockFile;
use parking_lot::deadlock;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use tauri::{Manager, RunEvent};
use tauri_nspanel::tauri_panel;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::{Target, TargetKind};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod api;
mod app;
mod chordpack;
mod constants;
pub mod git;
mod input;
mod mode;
mod models;
mod observables;
mod quickjs;
mod setup;
mod tauri_app;

pub use tauri_app::*;

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
    run_app();
}

pub fn run_app() {
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

    let env_log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let parsed_level = log::LevelFilter::from_str(&env_log_level).unwrap_or(log::LevelFilter::Info);

    let log_plugin = tauri_plugin_log::Builder::new()
        .clear_targets()
        .level(parsed_level)
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {}:{} - {}",
                record.level(),
                record
                    .file()
                    .and_then(|f| {
                        let path = std::path::Path::new(f);
                        let file_name = path.file_name()?.to_str()?;
                        let parent = path.parent()?.file_name()?.to_str()?;
                        Some(format!("{}/{}", parent, file_name))
                    })
                    .unwrap_or_else(|| "unknown".to_string()),
                record.line().unwrap_or(0),
                message
            ))
        })
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("chords".into()),
            }),
            Target::new(TargetKind::Webview),
        ])
        .build();

    let api = ApiImpl::default();
    let app = tauri::Builder::default()
        .invoke_handler(taurpc::create_ipc_handler(api.clone().into_handler()))
        .menu(|handle| tauri_app::menu::build_app_menu(handle))
        .on_menu_event(|handle, event| {
            tauri_app::menu::handle_menu_event(handle, &event);
        })
        .plugin(log_plugin)
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|handle, _args, _cwd| {
            let settings = handle.app_settings();
            if let Err(error) = settings.ui.open() {
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
            api.set_handle(app.handle().clone());
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
        if let RunEvent::Exit = event {
            if let Err(error) = handle.state::<AppLockFile>().cleanup() {
                log::error!("Failed to remove app lock file on exit: {error}");
            }
        }

        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = event {
            let settings = handle.app_settings();
            if let Err(error) = settings.ui.open() {
                log::error!("Failed to show settings window after dock reopen: {error}");
            }
        }
    });
}

pub async fn run_script(path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
    quickjs::run_standalone_module(path.as_ref()).await
}

pub async fn run_script_export(
    path: impl AsRef<std::path::Path>,
    export_name: impl Into<String>,
    args: Vec<String>,
) -> anyhow::Result<()> {
    quickjs::run_standalone_export(path.as_ref(), export_name.into(), args).await
}
