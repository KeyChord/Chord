use crate::app::chord_package_manager::LocalChordPackage;
use crate::app::global_hotkey_store::GlobalShortcutMappingInfo;
use crate::logging::AppLogEntry;
use crate::state::GitRepo;
use serde::Serialize;
use specta::Type;
use taurpc_macros::taurpc_api;

pub use crate::api::ApiImpl;

mod api;
pub use api::*;
mod error;
pub use error::*;

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChordCommandOutput {
    pub exit_code: Option<i32>,
    pub stderr: String,
    pub stdout: String,
}

#[taurpc_api(mod = "resolvers")]
pub trait Api {
    async fn open_accessibility_settings();
    async fn open_input_monitoring_settings();
    async fn refresh_permissions() -> AppResult<(bool, bool)>;
    async fn add_git_repo(repo: String) -> AppResult<GitRepo>;
    async fn reset_default_chords() -> AppResult<()>;
    async fn remove_git_repo(repo: String) -> AppResult<()>;
    async fn sync_git_repo(repo: String) -> AppResult<GitRepo>;
    async fn list_local_chord_folders() -> AppResult<Vec<String>>;
    async fn pick_local_chord_folder() -> AppResult<Option<String>>;
    async fn add_local_chord_folder(path: String) -> AppResult<LocalChordPackage>;
    async fn list_global_shortcut_mappings() -> AppResult<Vec<GlobalShortcutMappingInfo>>;
    async fn remove_global_shortcut_mapping(shortcut: String) -> AppResult<()>;
    async fn update_global_shortcut_mapping(
        old_shortcut: String,
        new_shortcut: String,
    ) -> AppResult<()>;
    async fn set_placeholder_chord_binding(
        file_path: String,
        sequence_template: String,
        sequence: String,
    ) -> AppResult<()>;
    async fn remove_placeholder_chord_binding(
        file_path: String,
        sequence_template: String,
    ) -> AppResult<()>;
    async fn relaunch_app(bundle_id: String) -> AppResult<()>;
    async fn toggle_autostart() -> AppResult<()>;
    async fn toggle_menu_bar_icon() -> AppResult<()>;
    async fn toggle_dock_icon() -> AppResult<()>;
    async fn toggle_hide_guide_by_default() -> AppResult<()>;
    async fn quit_app() -> AppResult<()>;
    async fn get_current_states() -> AppResult<String>;
    async fn get_app_logs() -> AppResult<Vec<AppLogEntry>>;
    async fn run_chord_command(command: String) -> AppResult<ChordCommandOutput>;
}
