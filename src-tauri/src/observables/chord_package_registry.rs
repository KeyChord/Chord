use crate::app::chord_package_manager::ChordPackage;
use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackageRegistryState {
    sorted_package_names_by_priority: Vec<String>
}

define_observable! {
    pub struct ChordPackageRegistryObservable(ChordPackageRegistryState);
    id: "chord-package-manager";
}
