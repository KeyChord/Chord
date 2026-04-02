use std::collections::HashMap;
use crate::app::chord_package_manager::ChordPackage;
use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;
use crate::app::chord_package_store::ChordPackageStoreEntry;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackageStoreState {
    pub entries: HashMap<String, ChordPackageStoreEntry>
}

define_observable! {
    pub struct ChordPackageStoreObservable(ChordPackageStoreState);
    id: "chord-package-store";
}
