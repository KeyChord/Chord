use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum AppModeState {
    #[default]
    Idle,
    Chord
}

define_observable!(
    pub struct AppModeObservable(AppModeState);
    id: "app-mode";
);
