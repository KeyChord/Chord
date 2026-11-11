use super::PinnedGitRepoSpec;
use crate::state::{GitRepo, PackageUpdate, PackageUpdateSource, installed_revision};
use std::collections::HashMap;
use std::path::PathBuf;

/// What an update check compares the installed packages with.
#[derive(Debug, Default)]
pub struct UpdateCheckPlan {
    /// Built-in packages installed at another commit than this Chord's pin. Found without the
    /// network.
    pub built_in: Vec<PackageUpdate>,
    /// Packages that follow their repository, whose newest commit has to be looked up.
    pub latest: Vec<LatestCandidate>,
}

#[derive(Debug)]
pub struct LatestCandidate {
    pub slug: String,
    pub url: String,
    /// The installed checkout, whose configuration the lookup uses.
    pub checkout: PathBuf,
    pub installed_rev: String,
}

impl LatestCandidate {
    /// The update to the repository's newest commit, `head`, if that isn't the installed one.
    pub fn update_to(self, head: String) -> Option<PackageUpdate> {
        (head != self.installed_rev).then(|| PackageUpdate {
            slug: self.slug,
            installed_rev: self.installed_rev,
            rev: head,
            source: PackageUpdateSource::Latest,
        })
    }
}

/// A package linked to a local folder runs from that folder, so it has no updates. One pinned to
/// a commit only has the update this Chord's pin gives it, if it's built in.
pub fn plan_update_check(
    repos: &HashMap<String, GitRepo>,
    defaults: &[PinnedGitRepoSpec],
) -> UpdateCheckPlan {
    let pins = defaults
        .iter()
        .map(|spec| (spec.repo_ref.slug().to_ascii_lowercase(), spec.rev.as_str()))
        .collect::<HashMap<_, _>>();
    let mut plan = UpdateCheckPlan::default();
    let mut repos = repos.values().collect::<Vec<_>>();
    repos.sort_by(|a, b| a.slug.cmp(&b.slug));
    for repo in repos {
        let Some(installed_rev) = installed_revision(repo) else {
            continue;
        };
        if repo.linked_local_path.is_some() {
            continue;
        }
        if repo.pinned_rev.is_none() {
            plan.latest.push(LatestCandidate {
                slug: repo.slug.clone(),
                url: repo.url.clone(),
                checkout: repo.local_abspath.clone(),
                installed_rev,
            });
        } else if let Some(pin) = pins.get(&repo.slug.to_ascii_lowercase())
            && let Ok(pin) = crate::git::normalize_revision(pin)
            && pin != installed_rev
        {
            plan.built_in.push(PackageUpdate {
                slug: repo.slug.clone(),
                installed_rev,
                rev: pin,
                source: PackageUpdateSource::BuiltIn,
            });
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitHubRepoRef;

    fn rev(c: char) -> String {
        c.to_string().repeat(40)
    }

    fn repo(name: &str, at: char, pinned: bool) -> GitRepo {
        let slug = format!("KeyChord/{name}");
        let mut repo: GitRepo = serde_json::from_value(serde_json::json!({
            "owner": "KeyChord", "name": name, "slug": slug,
            "url": format!("https://github.com/{slug}"),
            "localAbspath": format!("/packages/keychord/{}/{}", name.to_ascii_lowercase(), rev(at)),
            "headShortSha": null,
        }))
        .unwrap();
        repo.pinned_rev = pinned.then(|| rev(at));
        repo
    }

    fn spec(name: &str, at: char) -> PinnedGitRepoSpec {
        PinnedGitRepoSpec {
            repo_ref: GitHubRepoRef::parse(&format!("KeyChord/{name}")).unwrap(),
            rev: rev(at),
        }
    }

    #[test]
    fn built_in_packages_update_to_this_chords_pin_and_others_to_their_newest_commit() {
        let mut linked = repo("chords-linked", 'a', false);
        linked.linked_local_path = Some("/my checkout".into());
        let repos = [
            repo("chords-menu", 'a', true),
            repo("chords-com.apple.Safari", 'b', true),
            repo("chords-mine", 'a', true),
            repo("chords-raycast", 'a', false),
            linked,
        ]
        .into_iter()
        .map(|repo| (repo.slug.clone(), repo))
        .collect();
        let defaults = [
            spec("chords-menu", 'c'),
            spec("chords-com.apple.safari", 'b'),
        ];
        let plan = plan_update_check(&repos, &defaults);

        assert_eq!(
            plan.built_in,
            [PackageUpdate {
                slug: "KeyChord/chords-menu".into(),
                installed_rev: rev('a'),
                rev: rev('c'),
                source: PackageUpdateSource::BuiltIn,
            }]
        );
        let latest = plan.latest.into_iter().collect::<Vec<_>>();
        assert_eq!(
            latest.iter().map(|c| c.slug.as_str()).collect::<Vec<_>>(),
            ["KeyChord/chords-raycast"]
        );
        let candidate = latest.into_iter().next().unwrap();
        assert_eq!(candidate.url, "https://github.com/KeyChord/chords-raycast");
        assert_eq!(
            candidate.checkout,
            PathBuf::from(format!("/packages/keychord/chords-raycast/{}", rev('a')))
        );
        assert!(
            LatestCandidate {
                installed_rev: rev('a'),
                ..candidate_like("x")
            }
            .update_to(rev('a'))
            .is_none()
        );
        assert_eq!(
            candidate
                .update_to(rev('d'))
                .map(|update| (update.rev, update.source)),
            Some((rev('d'), PackageUpdateSource::Latest))
        );
    }

    fn candidate_like(slug: &str) -> LatestCandidate {
        LatestCandidate {
            slug: slug.into(),
            url: String::new(),
            checkout: PathBuf::new(),
            installed_rev: String::new(),
        }
    }
}
