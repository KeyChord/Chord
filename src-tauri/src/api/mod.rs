use std::sync::Arc;
use parking_lot::Mutex;
use tauri::AppHandle;
use crate::app::chord_package_registry::LocalChordPackage;
use crate::app::global_hotkey_store::GlobalShortcutMappingInfo;
use crate::observables::GitRepo;
use crate::startup::StartupStatusInfo;

mod error;
pub use error::*;
mod resolvers;

#[taurpc::procedures(export_to = "../src/api/bindings.gen.ts")]
pub trait Api {
    #[taurpc(alias = "openAccessibilitySettings")]
    async fn open_accessibility_settings();
    #[taurpc(alias = "openInputMonitoringSettings")]
    async fn open_input_monitoring_settings();
    #[taurpc(alias = "getStartupStatus")]
    async fn get_startup_status() -> AppResult<StartupStatusInfo>;
    #[taurpc(alias = "completeOnboarding")]
    async fn complete_onboarding() -> AppResult<()>;
    #[taurpc(alias = "addGitRepo")]
    async fn add_git_repo(repo: String) -> AppResult<GitRepo>;
    #[taurpc(alias = "syncGitRepo")]
    async fn sync_git_repo(repo: String) -> AppResult<GitRepo>;
    #[taurpc(alias = "listLocalChordFolders")]
    async fn list_local_chord_folders() -> AppResult<Vec<LocalChordPackage>>;
    #[taurpc(alias = "pickLocalChordFolder")]
    async fn pick_local_chord_folder() -> AppResult<Option<String>>;
    #[taurpc(alias = "addLocalChordFolder")]
    async fn add_local_chord_folder(path: String) -> AppResult<LocalChordPackage>;
    #[taurpc(alias = "listGlobalShortcutMappings")]
    async fn list_global_shortcut_mappings() -> AppResult<Vec<GlobalShortcutMappingInfo>>;
    #[taurpc(alias = "removeGlobalShortcutMapping")]
    async fn remove_global_shortcut_mapping(shortcut: String) -> AppResult<()>;
    #[taurpc(alias = "updateGlobalShortcutMapping")]
    async fn update_global_shortcut_mapping(
        old_shortcut: String,
        new_shortcut: String,
    ) -> AppResult<()>;
    #[taurpc(alias = "setPlaceholderChordBinding")]
    async fn set_placeholder_chord_binding(
        file_path: String,
        sequence_template: String,
        sequence: String,
    ) -> AppResult<()>;
    #[taurpc(alias = "removePlaceholderChordBinding")]
    async fn remove_placeholder_chord_binding(
        file_path: String,
        sequence_template: String,
    ) -> AppResult<()>;
    #[taurpc(alias = "relaunchApp")]
    async fn relaunch_app(bundle_id: String) -> AppResult<()>;
    #[taurpc(alias = "toggleAutostart")]
    async fn toggle_autostart() -> AppResult<()>;
    #[taurpc(alias = "toggleMenuBarIcon")]
    async fn toggle_menu_bar_icon() -> AppResult<()>;
    #[taurpc(alias = "toggleDockIcon")]
    async fn toggle_dock_icon() -> AppResult<()>;
    #[taurpc(alias = "toggleHideGuideByDefault")]
    async fn toggle_hide_guide_by_default() -> AppResult<()>;
    #[taurpc(alias = "quitApp")]
    async fn quit_app() -> AppResult<()>;
    #[taurpc(alias = "getCurrentStates")]
    async fn get_current_states() -> AppResult<String>;
}

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
