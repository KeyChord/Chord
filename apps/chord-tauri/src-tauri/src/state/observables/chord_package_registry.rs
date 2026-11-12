use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackageStoreState {
    /// The user's ranking of installed and built-in packages, by name, highest first.
    pub order: Vec<String>,
}

define_observable! {
    pub struct ChordPackageStoreObservable(ChordPackageStoreState);
    id: "chord-package-store";
}
