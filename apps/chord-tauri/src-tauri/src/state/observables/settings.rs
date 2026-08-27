use crate::define_observable;
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
}

impl Default for AppSettingsState {
    fn default() -> Self {
        Self {
            bundle_ids_needing_relaunch: vec![],
            show_menu_bar_icon: true,
            show_dock_icon: true,
            is_chord_panel_hidden_by_default: false,
        }
    }
}

define_observable! {
    pub struct AppSettingsObservable(AppSettingsState);
    id: "settings";
}
