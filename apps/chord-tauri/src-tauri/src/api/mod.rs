// `taurpc_api` expands to code containing `unwrap()` calls, attributed to the trait
// declarations below; the hand-written source in this module contains none.
#![allow(clippy::unwrap_used)]

use crate::app::chord_package_manager::{ChordsFileProblem, LocalChordPackage, PersonalPackages};
use crate::app::global_hotkey_store::GlobalShortcutMappingInfo;
use crate::logging::AppLogEntry;
use crate::models::ChordMacroName;
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

/// What first-run setup needs to know about this Mac.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingInfo {
    /// Chord is running from the disk image it was downloaded on, and would disappear with it.
    pub needs_move_to_applications: bool,
    /// What this version of macOS calls the System Settings page that lists Chord.
    pub accessibility_pane_name: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub bundle_id: String,
    pub name: String,
    pub icon_data_url: Option<String>,
    /// Kept in the Dock, so it is probably an app this person uses every day.
    pub in_dock: bool,
}

#[taurpc_api(mod = "resolvers")]
pub trait Api {
    async fn open_accessibility_settings();
    async fn request_accessibility_access();
    async fn get_onboarding_info() -> AppResult<OnboardingInfo>;
    async fn set_onboarding_complete(complete: bool) -> AppResult<()>;
    async fn move_to_applications() -> AppResult<()>;
    async fn list_installed_apps() -> AppResult<Vec<InstalledApp>>;
    async fn add_app_chord(sequence: String, name: String, bundle_id: String) -> AppResult<()>;
    async fn refresh_permissions() -> AppResult<bool>;
    async fn voice_over_uses_caps_lock() -> bool;
    async fn open_voice_over_utility();
    async fn add_git_repo(repo: String) -> AppResult<GitRepo>;
    async fn add_git_monorepo(repo: String) -> AppResult<GitRepo>;
    async fn list_monorepo_packages(source: String) -> AppResult<Vec<(String, bool)>>;
    async fn set_monorepo_packages(source: String, names: Vec<String>) -> AppResult<()>;
    async fn list_local_chord_monorepos() -> AppResult<Vec<String>>;
    async fn add_local_chord_monorepo(path: String) -> AppResult<()>;
    async fn remove_local_chord_monorepo(path: String) -> AppResult<()>;
    async fn reset_default_chords() -> AppResult<()>;
    async fn remove_git_repo(repo: String) -> AppResult<()>;
    async fn sync_git_repo(repo: String) -> AppResult<GitRepo>;
    async fn set_git_repo_local_link(repo: String, path: Option<String>) -> AppResult<GitRepo>;
    async fn reload_chords() -> AppResult<()>;
    async fn set_package_approvals(identities: Vec<String>, approved: bool) -> AppResult<()>;
    async fn set_chord_macro(
        target: ChordMacroName,
        sequence: String,
        previous_target: Option<ChordMacroName>,
    ) -> AppResult<()>;
    async fn remove_chord_macro(target: ChordMacroName) -> AppResult<()>;
    async fn list_local_chord_folders() -> AppResult<Vec<String>>;
    async fn pick_local_chord_folder() -> AppResult<Option<String>>;
    async fn add_local_chord_folder(path: String) -> AppResult<LocalChordPackage>;
    async fn list_personal_packages() -> AppResult<PersonalPackages>;
    async fn create_personal_package(dirname: String) -> AppResult<()>;
    async fn open_personal_package(dirname: Option<String>) -> AppResult<()>;
    async fn read_personal_chords_file(dirname: String, pathslug: String) -> AppResult<String>;
    async fn save_personal_chords_file(
        dirname: String,
        pathslug: String,
        contents: String,
        expected: Option<String>,
    ) -> AppResult<()>;
    async fn delete_personal_chords_file(dirname: String, pathslug: String) -> AppResult<()>;
    async fn check_chords_file(contents: String) -> AppResult<Option<ChordsFileProblem>>;
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
    async fn install_cli() -> AppResult<()>;
    async fn refresh_cli_installation() -> AppResult<()>;
    async fn toggle_autostart() -> AppResult<()>;
    async fn toggle_menu_bar_icon() -> AppResult<()>;
    async fn toggle_dock_icon() -> AppResult<()>;
    async fn toggle_hide_hint_panel_by_default() -> AppResult<()>;
    async fn toggle_settings_sidebar() -> AppResult<()>;
    async fn quit_app() -> AppResult<()>;
    async fn install_update() -> AppResult<()>;
    async fn restart_to_update() -> AppResult<()>;
    async fn get_current_states() -> AppResult<String>;
    async fn get_app_logs() -> AppResult<Vec<AppLogEntry>>;
    async fn run_chord_command(command: String) -> AppResult<ChordCommandOutput>;
}

#[cfg(test)]
mod generation {
    use super::*;

    /// Generate IPC types without starting the desktop app.
    #[test]
    #[ignore = "writes generated TypeScript bindings; run explicitly after changing the API"]
    fn export_typescript_bindings() {
        taurpc::Exporter::new()
            .export(
                &ApiImpl::default().into_handler(),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../packages/dev/improve/chord/api/taurpc/bindings.gen.ts"
                ),
            )
            .unwrap();
    }
}
