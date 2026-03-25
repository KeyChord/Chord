use crate::observables::GitRepo;
use crate::tauri_app::startup::StartupStatusInfo;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::AppHandle;
use crate::registry::LocalChordPackage;

mod resolvers;

mod error;
pub use error::*;

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct GlobalShortcutMappingInfo {
    pub shortcut: String,
    pub bundle_id: String,
    pub hotkey_id: String,
}

fn open_system_settings(url: &str, permission_name: &str) {
    if let Err(error) = std::process::Command::new("open").arg(url).spawn() {
        log::error!("Failed to open {permission_name} settings: {error}");
    }
}

fn normalize_placeholder_sequence(sequence: &str) -> AppResult<String> {
    let normalized = sequence.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AppError::Message(
            "placeholder sequence cannot be empty".into(),
        ));
    }

    if !normalized.chars().all(|ch| ch.is_ascii_lowercase()) {
        return Err(AppError::Message(
            "placeholder sequence must only contain letters a-z".into(),
        ));
    }

    Ok(normalized)
}

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

    fn handle(&self) -> AppResult<AppHandle> {
        self.handle
            .lock()
            .clone()
            .ok_or_else(|| AppError::Message("app handle is not initialized".to_string()))
    }
}

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn get_current_states(self) -> AppResult<String> {
        resolvers::get_current_states::get_current_states(self).await
    }

    async fn open_accessibility_settings(self) {
        resolvers::open_accessibility_settings::open_accessibility_settings(self).await
    }

    async fn toggle_autostart(self) -> AppResult<()> {
        resolvers::toggle_autostart::toggle_autostart(self).await
    }

    async fn toggle_menu_bar_icon(self) -> AppResult<()> {
        resolvers::toggle_menu_bar_icon::toggle_menu_bar_icon(self).await
    }

    async fn toggle_dock_icon(self) -> AppResult<()> {
        resolvers::toggle_dock_icon::toggle_dock_icon(self).await
    }

    async fn toggle_hide_guide_by_default(self) -> AppResult<()> {
        resolvers::toggle_hide_guide_by_default::toggle_hide_guide_by_default(self).await
    }

    async fn quit_app(self) -> AppResult<()> {
        resolvers::quit_app::quit_app(self).await
    }

    async fn open_input_monitoring_settings(self) {
        resolvers::open_input_monitoring_settings::open_input_monitoring_settings(self).await
    }

    async fn get_startup_status(self) -> AppResult<StartupStatusInfo> {
        resolvers::get_startup_status::get_startup_status(self).await
    }

    async fn complete_onboarding(self) -> AppResult<()> {
        resolvers::complete_onboarding::complete_onboarding(self).await
    }

    async fn add_git_repo(self, repo: String) -> AppResult<GitRepo> {
        resolvers::add_git_repo::add_git_repo(self, repo).await
    }

    async fn sync_git_repo(self, repo: String) -> AppResult<GitRepo> {
        resolvers::sync_git_repo::sync_git_repo(self, repo).await
    }

    async fn list_local_chord_folders(self) -> AppResult<Vec<LocalChordPackage>> {
        resolvers::list_local_chord_folders::list_local_chord_folders(self).await
    }

    async fn pick_local_chord_folder(self) -> AppResult<Option<String>> {
        resolvers::pick_local_chord_folder::pick_local_chord_folder(self).await
    }

    async fn add_local_chord_folder(self, path: String) -> AppResult<LocalChordPackage> {
        resolvers::add_local_chord_folder::add_local_chord_folder(self, path).await
    }

    async fn list_global_shortcut_mappings(self) -> AppResult<Vec<GlobalShortcutMappingInfo>> {
        resolvers::list_global_shortcut_mappings::list_global_shortcut_mappings(self).await
    }

    async fn remove_global_shortcut_mapping(self, shortcut: String) -> AppResult<()> {
        resolvers::remove_global_shortcut_mapping::remove_global_shortcut_mapping(self, shortcut)
            .await
    }

    async fn update_global_shortcut_mapping(
        self,
        old_shortcut: String,
        new_shortcut: String,
    ) -> AppResult<()> {
        resolvers::update_global_shortcut_mapping::update_global_shortcut_mapping(
            self,
            old_shortcut,
            new_shortcut,
        )
        .await
    }

    async fn set_placeholder_chord_binding(
        self,
        file_path: String,
        sequence_template: String,
        sequence: String,
    ) -> AppResult<()> {
        resolvers::set_placeholder_chord_binding::set_placeholder_chord_binding(
            self,
            file_path,
            sequence_template,
            sequence,
        )
        .await
    }

    async fn remove_placeholder_chord_binding(
        self,
        file_path: String,
        sequence_template: String,
    ) -> AppResult<()> {
        resolvers::remove_placeholder_chord_binding::remove_placeholder_chord_binding(
            self,
            file_path,
            sequence_template,
        )
        .await
    }

    async fn relaunch_app(self, bundle_id: String) -> AppResult<()> {
        resolvers::relaunch_app::relaunch_app(self, bundle_id).await
    }
}
