use tauri::AppHandle;
use crate::app::AppSingleton;
use crate::app::desktop_app::DesktopAppManager;
use crate::state::{DesktopAppManagerObservable, Observable};

impl AppSingleton<DesktopAppManagerObservable> for DesktopAppManager {
    fn new(handle: AppHandle) -> Self {
        Self {
            observable: DesktopAppManagerObservable::uninitialized(),
            handle,
        }
    }

    fn init(&self, observable: DesktopAppManagerObservable) -> anyhow::Result<()> {
        self.observable.init(observable);
        Ok(())
    }
}
