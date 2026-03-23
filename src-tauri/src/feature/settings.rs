use observable_property::ObservableProperty;
use std::sync::Arc;
use serde::Serialize;
use tauri::{App, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use typeshare::typeshare;
use crate::constants::SETTINGS_WINDOW_LABEL;
use crate::feature::{ChorderIndicatorUi, ChorderState};
use anyhow::Result;

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
    pub observable: SettingsObservable,
    pub ui: SettingsUi
}

pub struct SettingsUi {
    pub window: WebviewWindow,
}

impl SettingsUi {
    pub fn new(handle: AppHandle, state: &AppSettingsState) -> Result<Self> {
        let window = WebviewWindowBuilder::new(
            &handle,
            "settings",
            WebviewUrl::App("index.html".into()),
        )
            .title("Settings")
            .initialization_script(format!(r#"
              window.__SETTINGS_STATE__ = {}
            "#, serde_json::to_string(state)?))
            .inner_size(920.0, 760.0)
            .min_inner_size(760.0, 620.0)
            .visible(false)
            .resizable(true)
            .center()
            .build()?;

        Ok(Self { window })
    }
}



struct SettingsObservable {
    pub state: ObservableProperty<Arc<AppSettingsState>>
}

impl SettingsObservable {
    fn new(handle: AppHandle, state: AppSettingsState) -> Self {
        let state = ObservableProperty::new(Arc::new(state));

        if let Err(e) = state.subscribe(Arc::new(move |_, new_state| {
            if let Err(e) = handle.emit("app-settings-state-changed", new_state) {
                log::error!("Failed to emit app settings state change: {e}");
            }
        })) {
            log::error!("Failed to subscribe app settings state observer: {e}");
        };

        Self { state }
    }
}


impl AppSettings {
    pub fn new(handle: AppHandle) -> Result<Self> {
        let state = AppSettingsState::new();
        let ui = SettingsUi::new(handle.clone(), &state)?;
        let observable = SettingsObservable::new(handle.clone(), state);
        Ok(Self {
            observable,
            ui
        })
    }
}
