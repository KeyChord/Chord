use crate::define_observable;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use typeshare::typeshare;

#[typeshare]
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitReposState {
    pub repos: HashMap<String, GitRepo>,
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
    #[typeshare(serialized_as = "String")]
    pub local_abspath: PathBuf,
    pub head_short_sha: Option<String>,
    #[serde(default)]
    pub pinned_rev: Option<String>,
}

define_observable! {
    pub struct GitReposObservable(GitReposState);
    id: "git-repos";
}
