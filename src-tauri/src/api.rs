use crate::git::{GitHubRepoRef, GitRepoInfo};
use crate::tauri_app::store::GlobalHotkeyStore;
use crate::tauri_app::{
    list_active_chords, list_apps_needing_relaunch, list_loaded_chords,
    list_matching_chords, relaunch_app, reload_loaded_app_chords, startup, ActiveChordInfo,
    AppMetadataInfo, AppNeedsRelaunchInfo,
};
use crate::tauri_app::startup::StartupStatusInfo;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use crate::get_app_metadata;
use crate::tauri_app::git::{ChordPackageRegistry, LocalChordFolderInfo, LocalChordPackage};
use serde::Serialize;
use specta::Type;
use thiserror::Error;
use crate::feature::app_handle_ext::AppHandleExt;

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct GlobalShortcutMappingInfo {
    pub shortcut: String,
    pub bundle_id: String,
    pub hotkey_id: String,
}

fn global_hotkeys_store(app: &AppHandle) -> AppResult<GlobalHotkeyStore> {
    app.store("global-hotkeys.json")
        .map(GlobalHotkeyStore::new)
        .map_err(|error| AppError::Message(format!("failed to open global hotkeys store: {error}")))
}

fn open_system_settings(url: &str, permission_name: &str) {
    if let Err(error) = std::process::Command::new("open").arg(url).spawn() {
        log::error!("Failed to open {permission_name} settings: {error}");
    }
}

#[derive(Debug, Error, Serialize, Type)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Message(e.to_string())
    }
}

type AppResult<T> = Result<T, AppError>;

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
    #[taurpc(alias = "listGitRepos")]
    async fn list_git_repos() -> AppResult<Vec<GitRepoInfo>>;
    #[taurpc(alias = "addGitRepo")]
    async fn add_git_repo(repo: String) -> AppResult<GitRepoInfo>;
    #[taurpc(alias = "syncGitRepo")]
    async fn sync_git_repo(repo: String) -> AppResult<GitRepoInfo>;
    #[taurpc(alias = "listLocalChordFolders")]
    async fn list_local_chord_folders() -> AppResult<Vec<LocalChordPackage>>;
    #[taurpc(alias = "pickLocalChordFolder")]
    async fn pick_local_chord_folder() -> AppResult<Option<String>>;
    #[taurpc(alias = "addLocalChordFolder")]
    async fn add_local_chord_folder(path: String) -> AppResult<LocalChordPackage>;
    #[taurpc(alias = "listActiveChords")]
    async fn list_active_chords() -> AppResult<Vec<ActiveChordInfo>>;
    #[taurpc(alias = "listMatchingChords")]
    async fn list_matching_chords() -> AppResult<Vec<ActiveChordInfo>>;
    #[taurpc(alias = "getAppMetadata")]
    async fn get_app_metadata(bundle_id: String) -> AppResult<AppMetadataInfo>;
    #[taurpc(alias = "listRepoChords")]
    async fn list_repo_chords(repo: String) -> AppResult<Vec<ActiveChordInfo>>;
    #[taurpc(alias = "listLocalChordFolderChords")]
    async fn list_local_chord_folder_chords(path: String) -> AppResult<Vec<ActiveChordInfo>>;
    #[taurpc(alias = "listGlobalShortcutMappings")]
    async fn list_global_shortcut_mappings() -> AppResult<Vec<GlobalShortcutMappingInfo>>;
    #[taurpc(alias = "removeGlobalShortcutMapping")]
    async fn remove_global_shortcut_mapping(shortcut: String) -> AppResult<()>;
    #[taurpc(alias = "listAppsNeedingRelaunch")]
    async fn list_apps_needing_relaunch() -> AppResult<Vec<AppNeedsRelaunchInfo>>;
    #[taurpc(alias = "relaunchApp")]
    async fn relaunch_app(bundle_id: String) -> AppResult<()>;
}

#[derive(Clone, Default)]
pub struct ApiImpl {
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl ApiImpl {
    pub fn set_app_handle(&self, app_handle: AppHandle) {
        *self.app_handle.lock() = Some(app_handle);
    }

    fn app_handle(&self) -> AppResult<AppHandle> {
        self.app_handle
            .lock()
            .clone()
            .ok_or_else(|| AppError::Message("app handle is not initialized".to_string()))
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

    async fn get_startup_status(self) -> AppResult<StartupStatusInfo> {
        let handle = self.app_handle()?;
        Ok(startup::get_startup_status(&handle)?)
    }

    async fn complete_onboarding(self) -> AppResult<()> {
        let handle = self.app_handle()?;
        Ok(startup::complete_onboarding(&handle)?)
    }

    async fn list_git_repos(self) -> AppResult<Vec<GitRepoInfo>> {
        let handle = self.app_handle()?;
        let registry = handle.state::<ChordPackageRegistry>();
        Ok(registry.git.discover_repos()?)
    }

    async fn add_git_repo(self, repo: String) -> AppResult<GitRepoInfo> {
        let handle = self.app_handle()?;
        let registry = handle.state::<ChordPackageRegistry>();
        let repo_info = registry.git.add_repo(GitHubRepoRef::parse(&repo)?)?;
        reload_loaded_app_chords(handle).await?;
        Ok(repo_info)
    }

    async fn sync_git_repo(self, repo: String) -> AppResult<GitRepoInfo> {
        let app_handle = self.app_handle()?;
        let registry = app_handle.state::<ChordPackageRegistry>();
        let repo_info =
            registry.git.sync_repo(GitHubRepoRef::parse(&repo)?)?;
        reload_loaded_app_chords(app_handle).await?;
        Ok(repo_info)
    }

    async fn list_local_chord_folders(self) -> AppResult<Vec<LocalChordPackage>> {
        let app_handle = self.app_handle()?;
        let registry = app_handle.state::<ChordPackageRegistry>();
        Ok(registry .local .list()?)
    }

    async fn pick_local_chord_folder(self) -> AppResult<Option<String>> {
        let app_handle = self.app_handle()?;
        let registry = app_handle.app_chord_package_registry();
        Ok(registry
            .local
            .pick()
            .map(|folder| folder.map(|folder| folder.path().display().to_string()))?)
    }

    async fn add_local_chord_folder(self, path: String) -> AppResult<LocalChordPackage> {
        let app_handle = self.app_handle()?;
        let registry = app_handle.app_chord_package_registry();
        let folder_info = registry.local.add(&path)?;
        reload_loaded_app_chords(app_handle).await?;
        Ok(folder_info)
    }

    async fn list_active_chords(self) -> AppResult<Vec<ActiveChordInfo>> {
        let app_handle = self.app_handle()?;
        Ok(list_active_chords(app_handle)?)
    }

    async fn list_matching_chords(self) -> AppResult<Vec<ActiveChordInfo>> {
        let app_handle = self.app_handle()?;
        Ok(list_matching_chords(app_handle)?)
    }

    async fn get_app_metadata(
        self,
        bundle_id: String,
    ) -> AppResult<AppMetadataInfo> {
        Ok(get_app_metadata(bundle_id)?)
    }

    async fn list_repo_chords(self, repo: String) -> AppResult<Vec<ActiveChordInfo>> {
        let app_handle = self.app_handle()?;
        let registry = app_handle.app_chord_package_registry();
        let loaded_chords = registry.git.load_repo_chords(&repo)?;
        Ok(list_loaded_chords(&loaded_chords))
    }

    async fn list_local_chord_folder_chords(
        self,
        path: String,
    ) -> AppResult<Vec<ActiveChordInfo>> {
        let app_handle = self.app_handle()?;
        let registry = app_handle.app_chord_package_registry();
        let loaded_chords = registry.local.load_folder_chords(&path)?;
        Ok(list_loaded_chords(&loaded_chords))
    }

    async fn list_global_shortcut_mappings(self) -> AppResult<Vec<GlobalShortcutMappingInfo>> {
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

    async fn remove_global_shortcut_mapping(self, shortcut: String) -> AppResult<()> {
        let app_handle = self.app_handle()?;
        let trimmed_shortcut = shortcut.trim();
        if trimmed_shortcut.is_empty() {
            // TODO: fix
            return Ok(())
        }

        let store = global_hotkeys_store(&app_handle)?;
        store.remove(trimmed_shortcut);
        Ok(())
    }

    async fn list_apps_needing_relaunch(self) -> AppResult<Vec<AppNeedsRelaunchInfo>> {
        let app_handle = self.app_handle()?;
        Ok(list_apps_needing_relaunch(app_handle)?)
    }

    async fn relaunch_app(self, bundle_id: String) -> AppResult<()> {
        let app_handle = self.app_handle()?;
        Ok(relaunch_app(app_handle, &bundle_id)?)
    }
}
