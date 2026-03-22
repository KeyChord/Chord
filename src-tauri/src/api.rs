use crate::git::{add_git_repo, discover_git_repos, load_repo_chords, sync_git_repo, GitRepoInfo};
use crate::sources::{
    add_local_chord_folder, list_local_chord_folders, load_local_chord_folder_chords,
    pick_local_chord_folder, LocalChordFolderInfo,
};
use crate::tauri_app::store::GlobalHotkeyStore;
use crate::tauri_app::{
    list_active_chords, list_apps_needing_relaunch, list_loaded_chords, list_matching_chords,
    relaunch_app, reload_loaded_app_chords, ActiveChordInfo, AppNeedsRelaunchInfo,
};
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct GlobalShortcutMappingInfo {
    pub shortcut: String,
    pub bundle_id: String,
    pub hotkey_id: String,
}

fn global_hotkeys_store(app: &AppHandle) -> Result<GlobalHotkeyStore, String> {
    app.store("global-hotkeys.json")
        .map(GlobalHotkeyStore::new)
        .map_err(|error| format!("failed to open global hotkeys store: {error}"))
}

fn open_system_settings(url: &str, permission_name: &str) {
    if let Err(error) = std::process::Command::new("open").arg(url).spawn() {
        log::error!("Failed to open {permission_name} settings: {error}");
    }
}

#[taurpc::procedures(export_to = "../src/api/bindings.ts")]
pub trait Api {
    async fn open_accessibility_settings();
    async fn open_input_monitoring_settings();
    async fn list_git_repos() -> Result<Vec<GitRepoInfo>, String>;
    async fn add_git_repo(repo: String) -> Result<GitRepoInfo, String>;
    async fn sync_git_repo(repo: String) -> Result<GitRepoInfo, String>;
    async fn list_local_chord_folders() -> Result<Vec<LocalChordFolderInfo>, String>;
    async fn pick_local_chord_folder() -> Result<Option<String>, String>;
    async fn add_local_chord_folder(path: String) -> Result<LocalChordFolderInfo, String>;
    async fn list_active_chords() -> Result<Vec<ActiveChordInfo>, String>;
    async fn list_matching_chords() -> Result<Vec<ActiveChordInfo>, String>;
    async fn list_repo_chords(repo: String) -> Result<Vec<ActiveChordInfo>, String>;
    async fn list_local_chord_folder_chords(path: String) -> Result<Vec<ActiveChordInfo>, String>;
    async fn list_global_shortcut_mappings() -> Result<Vec<GlobalShortcutMappingInfo>, String>;
    async fn remove_global_shortcut_mapping(shortcut: String) -> Result<(), String>;
    async fn list_apps_needing_relaunch() -> Result<Vec<AppNeedsRelaunchInfo>, String>;
    async fn relaunch_app(bundle_id: String) -> Result<(), String>;
}

#[derive(Clone, Default)]
pub struct ApiImpl {
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl ApiImpl {
    pub fn set_app_handle(&self, app_handle: AppHandle) {
        *self.app_handle.lock() = Some(app_handle);
    }

    fn app_handle(&self) -> Result<AppHandle, String> {
        self.app_handle
            .lock()
            .clone()
            .ok_or_else(|| "app handle is not initialized".to_string())
    }
}

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn open_accessibility_settings(self) {
        open_system_settings(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            "accessibility",
        );
    }

    async fn open_input_monitoring_settings(self) {
        open_system_settings(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
            "input monitoring",
        );
    }

    async fn list_git_repos(self) -> Result<Vec<GitRepoInfo>, String> {
        let app_handle = self.app_handle()?;
        discover_git_repos(app_handle).map_err(|error| error.to_string())
    }

    async fn add_git_repo(self, repo: String) -> Result<GitRepoInfo, String> {
        let app_handle = self.app_handle()?;
        let repo_info =
            add_git_repo(app_handle.clone(), &repo).map_err(|error| error.to_string())?;
        reload_loaded_app_chords(app_handle)
            .await
            .map_err(|error| error.to_string())?;
        Ok(repo_info)
    }

    async fn sync_git_repo(self, repo: String) -> Result<GitRepoInfo, String> {
        let app_handle = self.app_handle()?;
        let repo_info =
            sync_git_repo(app_handle.clone(), &repo).map_err(|error| error.to_string())?;
        reload_loaded_app_chords(app_handle)
            .await
            .map_err(|error| error.to_string())?;
        Ok(repo_info)
    }

    async fn list_local_chord_folders(self) -> Result<Vec<LocalChordFolderInfo>, String> {
        let app_handle = self.app_handle()?;
        list_local_chord_folders(app_handle).map_err(|error| error.to_string())
    }

    async fn pick_local_chord_folder(self) -> Result<Option<String>, String> {
        let app_handle = self.app_handle()?;
        pick_local_chord_folder(app_handle).map_err(|error| error.to_string())
    }

    async fn add_local_chord_folder(self, path: String) -> Result<LocalChordFolderInfo, String> {
        let app_handle = self.app_handle()?;
        let folder_info =
            add_local_chord_folder(app_handle.clone(), &path).map_err(|error| error.to_string())?;
        reload_loaded_app_chords(app_handle)
            .await
            .map_err(|error| error.to_string())?;
        Ok(folder_info)
    }

    async fn list_active_chords(self) -> Result<Vec<ActiveChordInfo>, String> {
        let app_handle = self.app_handle()?;
        list_active_chords(app_handle).map_err(|error| error.to_string())
    }

    async fn list_matching_chords(self) -> Result<Vec<ActiveChordInfo>, String> {
        let app_handle = self.app_handle()?;
        list_matching_chords(app_handle).map_err(|error| error.to_string())
    }

    async fn list_repo_chords(self, repo: String) -> Result<Vec<ActiveChordInfo>, String> {
        let app_handle = self.app_handle()?;
        let loaded_chords =
            load_repo_chords(app_handle, &repo).map_err(|error| error.to_string())?;
        Ok(list_loaded_chords(&loaded_chords))
    }

    async fn list_local_chord_folder_chords(
        self,
        path: String,
    ) -> Result<Vec<ActiveChordInfo>, String> {
        let app_handle = self.app_handle()?;
        let loaded_chords =
            load_local_chord_folder_chords(app_handle, &path).map_err(|error| error.to_string())?;
        Ok(list_loaded_chords(&loaded_chords))
    }

    async fn list_global_shortcut_mappings(self) -> Result<Vec<GlobalShortcutMappingInfo>, String> {
        let app_handle = self.app_handle()?;
        let store = global_hotkeys_store(&app_handle)?;
        let mut mappings = store
            .entries()
            .into_iter()
            .map(|(shortcut, entry)| GlobalShortcutMappingInfo {
                shortcut,
                bundle_id: entry.bundle_id,
                hotkey_id: entry.hotkey_id,
            })
            .collect::<Vec<_>>();

        mappings.sort_by(|left, right| {
            left.bundle_id
                .cmp(&right.bundle_id)
                .then(left.hotkey_id.cmp(&right.hotkey_id))
                .then(left.shortcut.cmp(&right.shortcut))
        });

        Ok(mappings)
    }

    async fn remove_global_shortcut_mapping(self, shortcut: String) -> Result<(), String> {
        let app_handle = self.app_handle()?;
        let trimmed_shortcut = shortcut.trim();
        if trimmed_shortcut.is_empty() {
            return Err("shortcut cannot be empty".to_string());
        }

        let store = global_hotkeys_store(&app_handle)?;
        store.remove(trimmed_shortcut);
        Ok(())
    }

    async fn list_apps_needing_relaunch(self) -> Result<Vec<AppNeedsRelaunchInfo>, String> {
        let app_handle = self.app_handle()?;
        list_apps_needing_relaunch(app_handle).map_err(|error| error.to_string())
    }

    async fn relaunch_app(self, bundle_id: String) -> Result<(), String> {
        let app_handle = self.app_handle()?;
        relaunch_app(app_handle, &bundle_id).map_err(|error| error.to_string())
    }
}
