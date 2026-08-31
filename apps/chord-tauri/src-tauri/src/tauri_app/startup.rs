pub(crate) const APP_STATE_STORE_PATH: &str = "app-state.json";

pub fn is_autostart_launch() -> bool {
    std::env::args_os().any(|arg| arg == "--autostart")
}

/// macOS permission checks are async on Tauri's runtime; `tauri::async_runtime::block_on` must not
/// run on a Tokio worker (e.g. from an IPC handler), so we isolate checks on a short-lived runtime.
fn required_permissions_granted_sync() -> bool {
    match std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build permission-check runtime");
        rt.block_on(async {
            tauri_plugin_macos_permissions::check_input_monitoring_permission().await
                && tauri_plugin_macos_permissions::check_accessibility_permission().await
        })
    })
    .join()
    {
        Ok(granted) => granted,
        Err(_) => {
            log::warn!("permission check thread panicked; treating permissions as not granted");
            false
        }
    }
}

pub fn should_show_permission_dialog() -> bool {
    !is_autostart_launch() && !required_permissions_granted_sync()
}
