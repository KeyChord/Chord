use crate::chords::Shortcut;
use crate::feature::app_handle_ext::AppHandleExt;
use crate::feature::placeholder_chords::{PlaceholderChordStoreEntry, PlaceholderChordStoreKey};
use crate::get_app_metadata;
use crate::git::GitHubRepoRef;
use crate::observables::{GitRepo, get_all_observable_states};
use crate::tauri_app::registry::LocalChordPackage;
use crate::tauri_app::startup::StartupStatusInfo;
use crate::tauri_app::{
    AppMetadataInfo, AppNeedsRelaunchInfo, list_apps_needing_relaunch, relaunch_app, startup,
};
use parking_lot::Mutex;
use serde::Serialize;
use specta::Type;
use std::sync::Arc;
use tauri::AppHandle;
use thiserror::Error;

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
    #[taurpc(alias = "getAppMetadata")]
    async fn get_app_metadata(bundle_id: String) -> AppResult<AppMetadataInfo>;
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
    #[taurpc(alias = "listAppsNeedingRelaunch")]
    async fn list_apps_needing_relaunch() -> AppResult<Vec<AppNeedsRelaunchInfo>>;
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
    #[taurpc(alias = "getCurrentStates")]
    async fn get_current_states() -> AppResult<String>;
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
    async fn get_current_states(self) -> AppResult<String> {
        let handle = self.app_handle()?;
        let states = get_all_observable_states(handle.into())?;
        Ok(serde_json::to_string(&states).map_err(|_err| AppError::Message("what".into()))?)
    }

    async fn open_accessibility_settings(self) {
        open_system_settings(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            "accessibility",
        );
    }

    async fn toggle_autostart(self) -> AppResult<()> {
        let handle = self.app_handle()?;
        let permissions = handle.app_permissions();
        Ok(permissions.toggle_autostart()?)
    }

    async fn toggle_menu_bar_icon(self) -> AppResult<()> {
        let handle = self.app_handle()?;
        let settings = handle.app_settings();
        Ok(settings.toggle_menu_bar_icon()?)
    }

    async fn toggle_dock_icon(self) -> AppResult<()> {
        let handle = self.app_handle()?;
        let settings = handle.app_settings();
        Ok(settings.toggle_dock_icon()?)
    }

    async fn toggle_hide_guide_by_default(self) -> AppResult<()> {
        let handle = self.app_handle()?;
        let settings = handle.app_settings();
        Ok(settings.toggle_hide_guide_by_default()?)
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

    async fn add_git_repo(self, repo: String) -> AppResult<GitRepo> {
        let handle = self.app_handle()?;
        let store = handle.app_git_repos_store();
        let repo_ref = GitHubRepoRef::parse(&repo)?;
        store.add_repo(repo_ref.clone())?;

        let chord_registry = handle.app_chord_registry();
        chord_registry.reload().await?;
        Ok(repo_ref.into_repo(store.github_repos_dir()?.as_path()))
    }

    async fn sync_git_repo(self, repo: String) -> AppResult<GitRepo> {
        let handle = self.app_handle()?;
        let store = handle.app_git_repos_store();
        let repo_ref = GitHubRepoRef::parse(&repo)?;
        store.sync_repo(repo_ref.clone())?;

        let chord_registry = handle.app_chord_registry();
        chord_registry.reload().await?;
        Ok(repo_ref.into_repo(store.github_repos_dir()?.as_path()))
    }

    async fn list_local_chord_folders(self) -> AppResult<Vec<LocalChordPackage>> {
        let handle = self.app_handle()?;
        let registry = handle.app_chord_package_registry();
        Ok(registry.local.list()?)
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
        let handle = self.app_handle()?;
        let registry = handle.app_chord_package_registry();
        let folder_info = registry.local.add(&path)?;

        let chord_registry = handle.app_chord_registry();
        chord_registry.reload().await?;
        Ok(folder_info)
    }

    async fn get_app_metadata(self, bundle_id: String) -> AppResult<AppMetadataInfo> {
        Ok(get_app_metadata(bundle_id)?)
    }

    async fn list_global_shortcut_mappings(self) -> AppResult<Vec<GlobalShortcutMappingInfo>> {
        let handle = self.app_handle()?;
        let store = handle.app_global_hotkey_store();
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
        let handle = self.app_handle()?;
        let store = handle.app_global_hotkey_store();
        let trimmed_shortcut = shortcut.trim();
        if trimmed_shortcut.is_empty() {
            return Err(AppError::Message("cannot be empty".into()));
        }

        store.remove(trimmed_shortcut)?;
        Ok(())
    }

    async fn update_global_shortcut_mapping(
        self,
        old_shortcut: String,
        new_shortcut: String,
    ) -> AppResult<()> {
        let handle = self.app_handle()?;
        let store = handle.app_global_hotkey_store();
        let old_shortcut = old_shortcut.trim();
        let new_shortcut = new_shortcut.trim();

        if old_shortcut.is_empty() || new_shortcut.is_empty() {
            return Err(AppError::Message("shortcut cannot be empty".into()));
        }

        Shortcut::parse(new_shortcut).map_err(|err| {
            AppError::Message(format!("invalid shortcut {new_shortcut:?}: {err}"))
        })?;

        store.update_shortcut(old_shortcut, new_shortcut)?;
        Ok(())
    }

    async fn set_placeholder_chord_binding(
        self,
        file_path: String,
        sequence_template: String,
        sequence: String,
    ) -> AppResult<()> {
        let handle = self.app_handle()?;
        let store = handle.app_placeholder_chord_store();
        let key = PlaceholderChordStoreKey {
            file_path,
            sequence_template,
        };
        let entry = PlaceholderChordStoreEntry {
            sequence: normalize_placeholder_sequence(&sequence)?,
        };

        store.set(key, entry)?;
        handle.app_chord_registry().reload().await?;
        Ok(())
    }

    async fn remove_placeholder_chord_binding(
        self,
        file_path: String,
        sequence_template: String,
    ) -> AppResult<()> {
        let handle = self.app_handle()?;
        let store = handle.app_placeholder_chord_store();
        store.remove(&PlaceholderChordStoreKey {
            file_path,
            sequence_template,
        })?;
        handle.app_chord_registry().reload().await?;
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
