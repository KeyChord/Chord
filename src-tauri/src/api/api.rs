use std::sync::Arc;
use parking_lot::Mutex;
use tauri::AppHandle;
use crate::api::{AppError, AppResult};

#[derive(Clone, Default)]
pub struct ApiImpl {
    handle: Arc<Mutex<Option<AppHandle>>>,
}

impl ApiImpl {
    pub fn set_handle(&self, handle: AppHandle) {
        *self.handle.lock() = Some(handle);
    }

    pub fn handle(&self) -> AppResult<AppHandle> {
        self.handle
            .lock()
            .clone()
            .ok_or_else(|| AppError::Message("app handle is not initialized".to_string()))
    }
}
