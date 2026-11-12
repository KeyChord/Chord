use crate::app::chord_package_manager::{ChordPackage, PersonalPackageState, ThirdPartyPackage};
use crate::define_observable;
use serde::Serialize;
use std::collections::HashMap;
use typeshare::typeshare;

#[allow(dead_code)]
struct ChordPackageInfo {
    name: String,
}

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordPackageManagerState {
    /// Loaded packages, highest rank first: Chord's core package, personal packages, installed
    /// packages in the user's order, then built-in packages.
    pub packages: Vec<ChordPackage>,
    /// Names of Chord's built-in (default) packages, which rank below every installed package.
    pub builtin_packages: Vec<String>,
    /// Problems from the last reload, by package name. Files that failed are left out of their
    /// package, which otherwise stays loaded.
    pub errors: HashMap<String, Vec<String>>,
    /// The folders in `~/.chord/packages`, in priority order.
    pub personal_packages: Vec<PersonalPackageState>,
    /// Packages outside the official namespace, including those blocked pending approval.
    pub third_party_packages: Vec<ThirdPartyPackage>,
}

define_observable! {
    pub struct ChordPackageManagerObservable(ChordPackageManagerState);
    id: "chord-package-manager";
}
