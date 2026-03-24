use crate::chords::Chord;
use crate::define_observable;
use crate::input::Key;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontmostState {
    pub frontmost_app_bundle_id: Option<String>,
}

define_observable!(
    #[derive(Debug)]
    pub struct FrontmostObservable(FrontmostState);
    id: "frontmost";
);
