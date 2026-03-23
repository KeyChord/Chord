use tauri::{AppHandle, Manager, Runtime};
use crate::AppContext;
use crate::feature::{AppChorder, AppFrontmost, AppSettings};
use crate::tauri_app::git::ChordPackageRegistry;

pub trait AppHandleExt {
    fn app_settings(&self) -> &AppSettings;
    fn app_chorder(&self) -> &AppChorder;
    fn app_context(&self) -> &AppContext;
    fn app_chord_package_registry(&self) -> &ChordPackageRegistry;
    fn app_frontmost(&self) -> &AppFrontmost;
}

impl<R: Runtime> AppHandleExt for AppHandle<R> {
    fn app_settings(&self) -> &AppSettings {
        self.state::<AppSettings>().inner()
    }

    fn app_chorder(&self) -> &AppChorder {
        self.state::<AppChorder>().inner()
    }

    fn app_context(&self) -> &AppContext {
        self.state::<AppContext>().inner()
    }

    fn app_chord_package_registry(&self) -> &ChordPackageRegistry {
        self.state::<ChordPackageRegistry>().inner()
    }

    fn app_frontmost(&self) -> &AppFrontmost {
        self.state::<AppFrontmost>().inner()
    }
}
