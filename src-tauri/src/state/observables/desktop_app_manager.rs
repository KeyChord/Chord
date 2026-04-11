use crate::app::desktop_app::DesktopAppMetadata;
use crate::define_observable;
use serde::Serialize;
use std::collections::HashMap;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAppManagerState {
    pub apps_needing_relaunch: Vec<String>,
    pub apps_metadata: HashMap<String, DesktopAppMetadata>,
}

define_observable! {
    pub struct DesktopAppManagerObservable(DesktopAppManagerState);
    id: "desktop-app-manager";
}
