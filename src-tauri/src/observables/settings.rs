use serde::Serialize;
use typeshare::typeshare;
use crate::define_observable;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsState {
    pub bundle_ids_needing_relaunch: Vec<String>,
}

define_observable! {
    pub struct AppSettingsObservable(AppSettingsState);
    id: "permissions";
}
