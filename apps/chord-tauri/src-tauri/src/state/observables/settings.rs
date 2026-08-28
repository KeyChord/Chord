use crate::define_observable;
use crate::js_engine::JsEngine;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsState {
    pub bundle_ids_needing_relaunch: Vec<String>,
    pub show_menu_bar_icon: bool,
    pub show_dock_icon: bool,
    pub is_chord_panel_hidden_by_default: bool,
    /// Engine used for JS handlers; takes effect after a restart.
    pub js_engine: JsEngine,
    /// The engine this process is actually running with.
    pub active_js_engine: JsEngine,
    /// Whether this build can run Bun (`--features bun`).
    pub is_bun_engine_available: bool,
}

impl Default for AppSettingsState {
    fn default() -> Self {
        Self {
            bundle_ids_needing_relaunch: vec![],
            show_menu_bar_icon: true,
            show_dock_icon: true,
            is_chord_panel_hidden_by_default: false,
            js_engine: JsEngine::default(),
            active_js_engine: JsEngine::default(),
            is_bun_engine_available: crate::js_engine::bun_available(),
        }
    }
}

define_observable! {
    pub struct AppSettingsObservable(AppSettingsState);
    id: "settings";
}
