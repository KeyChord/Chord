use crate::app::chord_package_manager::ChordPackage;
use crate::app::chord_package_store::ChordPackageStoreEntry;
use crate::define_observable;
use serde::Serialize;
use std::collections::HashMap;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackageStoreState {
    pub entries: HashMap<String, ChordPackageStoreEntry>,
}

define_observable! {
    pub struct ChordPackageStoreObservable(ChordPackageStoreState);
    id: "chord-package-store";
}
