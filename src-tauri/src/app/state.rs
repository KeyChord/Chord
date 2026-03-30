use tauri::AppHandle;

pub trait StateSingleton {
    fn new(handle: AppHandle) -> Self;
}

