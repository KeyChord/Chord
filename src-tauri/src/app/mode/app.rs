use tauri::AppHandle;
use crate::app::AppSingleton;
use crate::app::mode::AppModeManager;
use crate::state::{AppModeObservable, Observable};

impl<T> AppSingleton<T> for AppModeManager {
    fn new(handle: AppHandle) -> Self {
        Self {
            atomic_state: super::app_mode::AtomicAppModeState::new(crate::app::mode::app_mode::AppMode::Idle),
            observable: AppModeObservable::uninitialized(),
            handle
        }
    }

    fn init(&self, observable: AppModeObservable) -> anyhow::Result<()> {
        self.observable.init(observable);
        Ok(())
    }

}