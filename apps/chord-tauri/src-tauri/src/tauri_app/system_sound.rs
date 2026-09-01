use tauri::AppHandle;

pub fn play_failure_sound(handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Err(error) = handle.run_on_main_thread(|| {
        objc2_app_kit::NSBeep();
    }) {
        log::warn!("Failed to play chord failure sound: {error}");
    }

    #[cfg(not(target_os = "macos"))]
    let _ = handle;
}
