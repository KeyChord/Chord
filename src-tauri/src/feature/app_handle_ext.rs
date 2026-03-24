use crate::AppContext;
use crate::chords::ChordRegistry;
use crate::feature::global_hotkey::GlobalHotkeyStore;
use crate::feature::repos::GitReposStore;
use crate::feature::{AppChorder, AppFrontmost, AppPermissions, AppSettings};
use crate::observables::{ChordRegistryState, Observable};
use crate::tauri_app::registry::ChordPackageRegistry;
use anyhow::Result;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime, State};

pub struct AppManaged {
    pub settings: AppSettings,
    pub chorder: AppChorder,
    pub context: AppContext,
    pub chord_package_registry: ChordPackageRegistry,
    pub frontmost: AppFrontmost,
    pub permissions: AppPermissions,
    pub global_hotkey_store: GlobalHotkeyStore,
    pub git_repos_store: GitReposStore,
    pub chord_registry: ChordRegistry,
}

impl AppManaged {
    pub fn register(self, handle: &AppHandle) {
        handle.manage(self.frontmost);
        handle.manage(self.permissions);
        handle.manage(self.settings);
        handle.manage(self.chorder);
        handle.manage(self.context);
        handle.manage(self.chord_package_registry);
        handle.manage(self.global_hotkey_store);
        handle.manage(self.git_repos_store);
        handle.manage(self.chord_registry);
    }
}

pub trait AppHandleExt {
    fn app_settings(&self) -> &AppSettings;
    fn app_chorder(&self) -> &AppChorder;
    fn app_context(&self) -> &AppContext;
    fn app_chord_package_registry(&self) -> &ChordPackageRegistry;
    fn app_chord_registry(&self) -> &ChordRegistry;
    fn app_frontmost(&self) -> &AppFrontmost;
    fn app_permissions(&self) -> &AppPermissions;
    fn app_global_hotkey_store(&self) -> &GlobalHotkeyStore;
    fn app_git_repos_store(&self) -> &GitReposStore;
    fn observable_state<T: Observable>(&self) -> Result<Arc<T::State>>;
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

    fn app_chord_registry(&self) -> &ChordRegistry {
        self.state::<ChordRegistry>().inner()
    }

    fn app_frontmost(&self) -> &AppFrontmost {
        self.state::<AppFrontmost>().inner()
    }

    fn app_permissions(&self) -> &AppPermissions {
        self.state::<AppPermissions>().inner()
    }

    fn app_global_hotkey_store(&self) -> &GlobalHotkeyStore {
        self.state::<GlobalHotkeyStore>().inner()
    }

    fn app_git_repos_store(&self) -> &GitReposStore {
        self.state::<GitReposStore>().inner()
    }

    fn observable_state<T: Observable>(&self) -> Result<Arc<T::State>> {
        Ok(self.state::<Arc<T>>().get_state()?)
    }
}
