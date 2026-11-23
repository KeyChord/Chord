use crate::app::AppHandleExt;
use crate::desktop_platform::{self, DesktopProcess};
use tauri::AppHandle;

#[derive(Debug, Clone)]
pub struct AppIdentity {
    executable: String,
    window_class: Option<String>,
}

impl AppIdentity {
    pub(crate) fn from_process(process: &DesktopProcess) -> Self {
        Self {
            executable: process.executable.clone(),
            window_class: process.window_class.clone(),
        }
    }

    pub fn resolve(&self, handle: &AppHandle) -> Option<String> {
        let packages = handle.app_state().chord_package_manager();
        self.window_class
            .as_deref()
            .and_then(|class| packages.app_id_for_executable(class))
            .or_else(|| packages.app_id_for_executable(&self.executable))
            .or_else(|| Some(desktop_platform::executable_id(&self.executable)))
    }
}

pub(crate) fn process_ids(handle: &AppHandle, app_id: &str) -> Vec<i32> {
    desktop_platform::processes()
        .iter()
        .filter_map(|process| {
            (AppIdentity::from_process(process)
                .resolve(handle)
                .as_deref()
                == Some(app_id))
            .then_some(process.pid as i32)
        })
        .collect()
}
