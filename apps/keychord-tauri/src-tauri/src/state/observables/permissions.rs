use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppPermissionsState {
    pub is_autostart_enabled: Option<bool>,
    pub is_input_monitoring_enabled: Option<bool>,
    pub is_accessibility_enabled: Option<bool>,
}

define_observable! {
    pub struct AppPermissionsObservable(AppPermissionsState);
    id: "permissions";
}
