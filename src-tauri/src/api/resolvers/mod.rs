use crate::observables::GitRepo;
use crate::tauri_app::startup::StartupStatusInfo;
use crate::api::AppResult;
use crate::app::chord_package_registry::LocalChordPackage;
use crate::app::global_hotkey_store::GlobalShortcutMappingInfo;

macro_rules! taurpc_resolvers {
    (
        $(
            $name:ident ( $( $arg:ident : $arg_ty:ty ),* $(,)? ) $( -> $ret:ty )? ;
        )*
    ) => {
        $(
            pub mod $name;
        )*

        #[taurpc::resolvers]
        impl $crate::api::Api for $crate::api::ApiImpl {
            $(
                async fn $name(self, $( $arg : $arg_ty ),* ) $( -> $ret )? {
                    $crate::api::resolvers::$name::$name(self, $( $arg ),*).await
                }
            )*
        }
    };
}

taurpc_resolvers! {
    open_accessibility_settings();
    open_input_monitoring_settings();
    get_startup_status() -> AppResult<StartupStatusInfo>;
    complete_onboarding() -> AppResult<()>;
    add_git_repo(repo: String) -> AppResult<GitRepo>;
    sync_git_repo(repo: String) -> AppResult<GitRepo>;
    list_local_chord_folders() -> AppResult<Vec<LocalChordPackage>>;
    pick_local_chord_folder() -> AppResult<Option<String>>;
    add_local_chord_folder(path: String) -> AppResult<LocalChordPackage>;
    list_global_shortcut_mappings() -> AppResult<Vec<GlobalShortcutMappingInfo>>;
    remove_global_shortcut_mapping(shortcut: String) -> AppResult<()>;
    update_global_shortcut_mapping(
        old_shortcut: String,
        new_shortcut: String
    ) -> AppResult<()>;
    set_placeholder_chord_binding(
        file_path: String,
        sequence_template: String,
        sequence: String
    ) -> AppResult<()>;
    remove_placeholder_chord_binding(
        file_path: String,
        sequence_template: String
    ) -> AppResult<()>;
    relaunch_app(bundle_id: String) -> AppResult<()>;
    toggle_autostart() -> AppResult<()>;
    toggle_menu_bar_icon() -> AppResult<()>;
    toggle_dock_icon() -> AppResult<()>;
    toggle_hide_guide_by_default() -> AppResult<()>;
    quit_app() -> AppResult<()>;
    get_current_states() -> AppResult<String>;
}
