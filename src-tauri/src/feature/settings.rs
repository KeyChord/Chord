use observable_property::ObservableProperty;
use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, WebviewUrl, WebviewWindow};
use typeshare::typeshare;
use crate::feature::SafeAppHandle;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsState {
    pub bundle_ids_needing_relaunch: Vec<String>,
    pub git_repos: Vec<AppSettingsStateGitRepo>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsStateGitRepo {
    pub owner: String,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub local_path: String,
    pub head_short_sha: Option<String>
}


pub struct AppSettings {
    _observable: SettingsObservable,
    pub ui: SettingsUi
}

pub struct SettingsUi {
    pub window: WebviewWindow,
}

impl SettingsUi {
    pub fn new(handle: SafeAppHandle, state: &AppSettingsState) -> Result<Self> {
        let window = handle
            .new_webview_window_builder("settings", WebviewUrl::App("index.html".into()))
            .title("Settings")
            .initialization_script(format!(r#"
              window.__INITIAL_STATE__ = {}
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
    _state: ObservableProperty<Arc<AppSettingsState>>
}

impl SettingsObservable {
    fn new(ui: &SettingsUi, state: AppSettingsState) -> Self {
        let state = ObservableProperty::new(Arc::new(state));

        let window = ui.window.clone();
        if let Err(e) = state.subscribe(Arc::new(move |_, new_state| {
            if let Err(e) = window.emit("state:settings", new_state) {
                log::error!("Failed to emit app settings state change: {e}");
            }
        })) {
            log::error!("Failed to subscribe app settings state observer: {e}");
        };

        Self { _state: state }
    }
}


impl AppSettings {
    pub fn new(handle: SafeAppHandle, state: AppSettingsState) -> Result<Self> {
        let ui = SettingsUi::new(handle.clone(), &state)?;
        let observable = SettingsObservable::new(&ui, state);
        Ok(Self {
            _observable: observable,
            ui
        })
    }
}
