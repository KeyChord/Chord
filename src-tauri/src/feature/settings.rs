use observable_property::ObservableProperty;
use std::sync::Arc;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use typeshare::typeshare;
use crate::feature::{ChorderIndicatorUi, ChorderState};

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsState {
    bundle_ids_needing_relaunch: Vec<String>,
    git_repos: Vec<AppSettingsStateGitRepo>,
    permissions: AppPermissionsState
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsStateGitRepo {
    owner: String,
    name: String,
    slug: String,
    url: String,
    local_path: String,
    head_short_sha: Option<String>
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPermissionsState {
    is_autostart_enabled: bool,
    is_input_monitoring_enabled: bool,
    is_accessibility_enabled: bool,
}

impl AppPermissionsState {
    pub fn new() -> Self {
        Self {
            is_autostart_enabled: false,
            is_input_monitoring_enabled: false,
            is_accessibility_enabled: false,
        }
    }
}

impl AppSettingsState {
    pub fn new() -> Self {
        Self {
            bundle_ids_needing_relaunch: Vec::new(),
            git_repos: Vec::new(),
            permissions: AppPermissionsState::new(),
        }
    }
}

pub struct AppSettings {
    pub state: ObservableProperty<Arc<AppSettingsState>>,
}

impl AppSettings {
    pub fn new(handle: AppHandle) -> Self {
        let state = ObservableProperty::new(Arc::new(AppSettingsState::new()));
        if let Err(e) = state.subscribe(Arc::new(move |_, new_state| {
            if let Err(e) = handle.emit("app-settings-state-changed", new_state) {
                log::error!("Failed to emit app settings state change: {e}");
            }
        })) {
            log::error!("Failed to subscribe app settings state observer: {e}");
        };

        Self {
            state,
        }
    }
}