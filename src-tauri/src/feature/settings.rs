use observable_property::ObservableProperty;
use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, WebviewUrl, WebviewWindow};
use typeshare::typeshare;
use crate::feature::SafeAppHandle;
use crate::observables::{AppSettingsObservable, AppSettingsState};

pub struct AppSettings {
    _observable: AppSettingsObservable,
    pub ui: SettingsUi
}

pub struct SettingsUi {
    pub window: WebviewWindow,
}

impl SettingsUi {
    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let window = handle
            .new_webview_window_builder("settings", WebviewUrl::App("index.html".into()))?
            .title("Settings")
            .inner_size(920.0, 760.0)
            .min_inner_size(760.0, 620.0)
            .visible(false)
            .resizable(true)
            .center()
            .build()?;

        Ok(Self { window })
    }
}

impl AppSettings {
    pub fn new(handle: SafeAppHandle, state: AppSettingsState) -> Result<Self> {
        let ui = SettingsUi::new(handle.clone())?;
        let observable = AppSettingsObservable::new(handle, state)?;
        Ok(Self {
            _observable: observable,
            ui
        })
    }
}

