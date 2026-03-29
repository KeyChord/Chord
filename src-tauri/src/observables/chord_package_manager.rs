use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;
use crate::app::chord_package_manager::ChordPackage;

struct ChordPackageInfo {
    name: String
}

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackageManagerState {
    pub packages: Vec<ChordPackage>,
}

define_observable! {
    pub struct ChordPackageManagerObservable(ChordPackageManagerState);
    id: "chord-package-manager";
}
