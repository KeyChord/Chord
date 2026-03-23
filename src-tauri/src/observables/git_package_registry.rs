use serde::Serialize;
use typeshare::typeshare;
use crate::define_observable;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitPackageRegistryState {
    pub git_repos: Vec<GitRepo>
}

#[taurpc::ipc_type]
#[typeshare]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct GitRepo {
    pub owner: String,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub local_path: String,
    pub head_short_sha: Option<String>
}

define_observable! {
    pub struct GitPackageRegistryObservable(GitPackageRegistryState);
    id: "git-package-registry";
}
