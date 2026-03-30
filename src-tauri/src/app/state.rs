use std::sync::Arc;
use tauri::AppHandle;
use crate::observables::Observable;

pub trait StateSingleton {
    fn new(handle: AppHandle) -> Self;
}

