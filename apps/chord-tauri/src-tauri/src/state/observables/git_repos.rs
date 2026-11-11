use crate::define_observable;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use typeshare::typeshare;

#[typeshare]
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitReposState {
    pub repos: HashMap<String, GitRepo>,
    /// Newer revisions of installed packages, by repository slug. Chord never installs these by
    /// itself; Browse lists them for the user to install.
    pub updates: HashMap<String, PackageUpdate>,
    pub update_check: PackageUpdateCheck,
}

impl GitReposState {
    /// The state with `repos` installed, dropping updates for packages that were removed or
    /// moved off the revision the update was found for.
    pub fn with_repos(self, repos: HashMap<String, GitRepo>) -> Self {
        let mut updates = self.updates;
        updates.retain(|slug, update| {
            repos.get(slug).is_some_and(|repo| {
                repo.linked_local_path.is_none()
                    && installed_revision(repo).as_deref() == Some(update.installed_rev.as_str())
            })
        });
        Self {
            repos,
            updates,
            update_check: self.update_check,
        }
    }
}

/// The full commit ID a repository's checkout is at. Checkouts live in directories named after it.
pub fn installed_revision(repo: &GitRepo) -> Option<String> {
    repo.local_abspath
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| crate::git::normalize_revision(name).ok())
}

#[taurpc::ipc_type]
#[typeshare]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct GitRepo {
    pub owner: String,
    pub name: String,
    pub slug: String,
    pub url: String,
    #[typeshare(serialized_as = "String")]
    pub local_abspath: PathBuf,
    #[serde(default)]
    #[typeshare(serialized_as = "Option<String>")]
    pub linked_local_path: Option<PathBuf>,
    #[serde(default)]
    pub is_monorepo: bool,
    pub head_short_sha: Option<String>,
    #[serde(default)]
    pub pinned_rev: Option<String>,
}

#[typeshare]
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageUpdate {
    pub slug: String,
    /// The commit installed now.
    pub installed_rev: String,
    /// The commit updating moves to.
    pub rev: String,
    pub source: PackageUpdateSource,
}

#[typeshare]
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PackageUpdateSource {
    /// A built-in package this version of Chord pins to another commit.
    BuiltIn,
    /// An installed package whose repository has a newer commit.
    Latest,
}

#[typeshare]
#[derive(Debug, Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageUpdateCheck {
    pub checking: bool,
    /// When the last check finished, in milliseconds since the Unix epoch.
    pub checked_at: Option<f64>,
    /// Packages whose repository the last check couldn't reach.
    pub failed: Vec<String>,
}

define_observable! {
    pub struct GitReposObservable(GitReposState);
    id: "git-repos";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(slug: &str, rev: &str) -> GitRepo {
        serde_json::from_value(serde_json::json!({
            "owner": "KeyChord", "name": slug, "slug": slug, "url": "",
            "localAbspath": format!("/packages/keychord/{slug}/{rev}"), "headShortSha": null,
        }))
        .unwrap()
    }

    fn update(slug: &str, installed_rev: &str) -> PackageUpdate {
        PackageUpdate {
            slug: slug.into(),
            installed_rev: installed_rev.into(),
            rev: "b".repeat(40),
            source: PackageUpdateSource::Latest,
        }
    }

    #[test]
    fn installing_repos_keeps_only_updates_for_the_revisions_they_were_found_for() {
        let old = "a".repeat(40);
        let state = GitReposState {
            updates: ["kept", "moved", "removed", "linked"]
                .into_iter()
                .map(|slug| (slug.to_string(), update(slug, &old)))
                .collect(),
            ..Default::default()
        };
        let mut linked = repo("linked", &old);
        linked.linked_local_path = Some("/my checkout".into());
        let repos = [repo("kept", &old), repo("moved", &"b".repeat(40)), linked]
            .into_iter()
            .map(|repo| (repo.slug.clone(), repo))
            .collect();
        let next = state.with_repos(repos);
        assert_eq!(next.updates.keys().collect::<Vec<_>>(), ["kept"]);
    }
}
