use serde::Serialize;
use typeshare::typeshare;
use crate::define_observable;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppPermissionsState {
    pub is_autostart_enabled: bool,
    pub is_input_monitoring_enabled: bool,
    pub is_accessibility_enabled: bool,
}

define_observable! {
    pub struct AppPermissionsObservable(AppPermissionsState);
    id: "permissions";
}
