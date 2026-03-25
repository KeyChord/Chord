use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAppManagerState {
    pub apps_needing_relaunch: Vec<String>
}

define_observable! {
    pub struct DesktopAppManagerObservable(DesktopAppManagerState);
    id: "desktop-app-manager";
}
