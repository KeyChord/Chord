use crate::app::chord_package_manager::git::PinnedGitRepoSpec;
use crate::git::GitHubRepoRef;
use anyhow::Context;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Deserialize)]
struct DefaultPackagesFile {
    packages: BTreeMap<String, DefaultPackageSource>,
    /// The default sets earlier versions of Chord installed, newest first, as repository slug to
    /// revision.
    #[serde(default)]
    previous: Vec<BTreeMap<String, String>>,
}

#[derive(Deserialize)]
struct DefaultPackageSource {
    git: String,
    rev: String,
}

/// What Chord does to an install's built-in packages when it starts. It never moves an installed
/// package to another commit: Browse lists those updates for the user to install.
#[derive(Debug, Default)]
pub struct DefaultPackagePlan {
    /// Built-in packages this install hasn't had before.
    pub install: Vec<PinnedGitRepoSpec>,
    /// Packages that are no longer built in, still at a commit Chord installed.
    pub remove: Vec<String>,
    /// Every built-in package the install has now had, as lowercase slugs. Saved, so that one the
    /// user removes stays removed.
    pub offered: Vec<String>,
}

fn default_packages_file() -> anyhow::Result<DefaultPackagesFile> {
    #[cfg(debug_assertions)]
    {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/default-packages.toml");
        if path.exists() {
            log::debug!("Loading default packages from disk: {}", path.display());
            let contents = std::fs::read_to_string(path)?;
            return parse_file(&contents);
        }
    }

    parse_file(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/default-packages.toml"
    )))
}

fn parse_file(contents: &str) -> anyhow::Result<DefaultPackagesFile> {
    toml::from_str::<DefaultPackagesFile>(contents).context("Failed to parse the default packages")
}

fn pinned_specs(file: DefaultPackagesFile) -> anyhow::Result<Vec<PinnedGitRepoSpec>> {
    file.packages
        .into_iter()
        .map(|(package_name, source)| {
            let repo_ref = GitHubRepoRef::parse(&source.git).with_context(|| {
                format!("Package {package_name} must reference a GitHub repo URL")
            })?;

            Ok(PinnedGitRepoSpec {
                repo_ref,
                rev: source.rev,
            })
        })
        .collect()
}

pub fn load_default_packages() -> anyhow::Result<Vec<PinnedGitRepoSpec>> {
    pinned_specs(default_packages_file()?)
}

/// The package names of Chord's current defaults, sorted. These built-in packages rank below
/// every package the user installed, wherever it came from.
pub fn builtin_package_names() -> anyhow::Result<Vec<String>> {
    Ok(default_packages_file()?.packages.into_keys().collect())
}

/// The plan for an install whose repositories are these `(slug, pinned revision)` pairs, and that
/// has had the `offered` built-in packages (`None` when no Chord has saved them yet).
pub fn plan_default_packages(
    installed: &[(String, Option<String>)],
    offered: Option<&[String]>,
) -> anyhow::Result<DefaultPackagePlan> {
    let file = default_packages_file()?;
    let previous = file.previous.clone();
    Ok(plan(installed, offered, &pinned_specs(file)?, &previous))
}

fn plan(
    installed: &[(String, Option<String>)],
    offered: Option<&[String]>,
    current: &[PinnedGitRepoSpec],
    previous: &[BTreeMap<String, String>],
) -> DefaultPackagePlan {
    let key = |slug: &str| slug.to_ascii_lowercase();
    let previous = previous
        .iter()
        .map(|set| {
            set.iter()
                .map(|(slug, rev)| (key(slug), rev.as_str()))
                .collect::<HashMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let current_by_slug = current
        .iter()
        .map(|spec| (key(&spec.repo_ref.slug()), spec))
        .collect::<BTreeMap<_, _>>();
    let pins = installed
        .iter()
        .filter_map(|(slug, rev)| Some((key(slug), rev.as_deref()?)))
        .collect::<Vec<_>>();
    let installed_slugs = installed
        .iter()
        .map(|(slug, _)| key(slug))
        .collect::<HashSet<_>>();

    let offered = match offered {
        Some(offered) => offered
            .iter()
            .map(|slug| key(slug))
            .collect::<BTreeSet<_>>(),
        // Saved since Chord stopped moving defaults to new pins. Until then it kept them all on
        // one set, so the install has had that set's packages. Without a match it's had them all.
        None => install_generation(&pins, &previous)
            .map(|set| set.keys().cloned().collect())
            .unwrap_or_else(|| current_by_slug.keys().cloned().collect()),
    };

    let mut plan = DefaultPackagePlan::default();
    for (slug, spec) in &current_by_slug {
        if !offered.contains(slug) && !installed_slugs.contains(slug) {
            plan.install.push((*spec).clone());
        }
    }
    for (slug, rev) in installed {
        let was_default = rev
            .as_deref()
            .is_some_and(|rev| previous.iter().any(|set| set.get(&key(slug)) == Some(&rev)));
        if was_default && !current_by_slug.contains_key(&key(slug)) {
            plan.remove.push(slug.clone());
        }
    }
    plan.remove.sort();
    plan.offered = offered
        .into_iter()
        .chain(current_by_slug.into_keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    plan
}

/// The newest earlier default set that agrees with every pin it shares with the install.
fn install_generation<'a, 'b>(
    pins: &[(String, &str)],
    previous: &'a [HashMap<String, &'b str>],
) -> Option<&'a HashMap<String, &'b str>> {
    previous.iter().find(|set| {
        let shared = pins
            .iter()
            .filter_map(|(slug, rev)| Some((set.get(slug)?, rev)))
            .collect::<Vec<_>>();
        !shared.is_empty() && shared.iter().all(|(pin, rev)| pin == rev)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_default_packages_parse() {
        let packages = load_default_packages().unwrap();
        assert!(
            packages
                .iter()
                .any(|package| package.repo_ref.slug() == "KeyChord/chords-chord")
        );
        assert!(!default_packages_file().unwrap().previous.is_empty());
    }

    fn spec(name: &str, rev: &str) -> PinnedGitRepoSpec {
        PinnedGitRepoSpec {
            repo_ref: GitHubRepoRef::parse(&format!("https://github.com/KeyChord/{name}")).unwrap(),
            rev: rev.into(),
        }
    }

    fn set(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(name, rev)| (format!("KeyChord/{name}"), rev.to_string()))
            .collect()
    }

    fn installed(entries: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
        entries
            .iter()
            .map(|(name, rev)| (format!("KeyChord/{name}"), rev.map(str::to_string)))
            .collect()
    }

    fn names(specs: &[PinnedGitRepoSpec]) -> Vec<String> {
        specs
            .iter()
            .map(|spec| format!("{}@{}", spec.repo_ref.name, spec.rev))
            .collect()
    }

    fn slugs(names: &[&str]) -> Vec<String> {
        names
            .iter()
            .map(|name| format!("keychord/{}", name.to_ascii_lowercase()))
            .collect()
    }

    fn fixture() -> (Vec<PinnedGitRepoSpec>, Vec<BTreeMap<String, String>>) {
        let current = vec![
            spec("chords-chord", "c3"),
            spec("chords-menu", "m3"),
            spec("chords-com.apple.finder", "f3"),
            spec("chords-com.apple.Safari", "s3"),
        ];
        let previous = vec![
            set(&[
                ("chords-chord", "c2"),
                ("chords-menu", "m2"),
                ("chords-com.apple.finder", "f2"),
                ("chords-com.apple.Safari", "s2"),
            ]),
            set(&[
                ("chords-chord", "c1"),
                ("chords-menu", "m1"),
                ("chords-os", "o1"),
            ]),
        ];
        (current, previous)
    }

    const ALL: &[&str] = &[
        "chords-chord",
        "chords-com.apple.finder",
        "chords-com.apple.Safari",
        "chords-menu",
    ];

    #[test]
    fn a_first_run_installs_every_default() {
        let (current, previous) = fixture();
        let plan = plan(&[], Some(&[]), &current, &previous);
        assert_eq!(
            names(&plan.install),
            [
                "chords-chord@c3",
                "chords-com.apple.finder@f3",
                "chords-com.apple.Safari@s3",
                "chords-menu@m3"
            ]
        );
        assert!(plan.remove.is_empty());
        assert_eq!(plan.offered, slugs(ALL));
    }

    #[test]
    fn installed_packages_keep_their_commits() {
        let (current, previous) = fixture();
        let plan = plan(
            &installed(&[
                ("chords-chord", Some("c1")),
                ("chords-menu", Some("m1")),
                ("chords-os", Some("o1")),
                ("chords-terminal", None),
            ]),
            None,
            &current,
            &previous,
        );
        // Set up with the oldest set: it gets what the defaults gained, and loses chords-os.
        assert_eq!(
            names(&plan.install),
            ["chords-com.apple.finder@f3", "chords-com.apple.Safari@s3"]
        );
        assert_eq!(plan.remove, ["KeyChord/chords-os"]);
        assert_eq!(
            plan.offered,
            slugs(&[
                "chords-chord",
                "chords-com.apple.finder",
                "chords-com.apple.Safari",
                "chords-menu",
                "chords-os"
            ])
        );
    }

    #[test]
    fn a_default_the_user_removed_stays_removed() {
        let (current, previous) = fixture();
        // Set up with the newer set, which had Safari, and the user removed it.
        let before = installed(&[
            ("chords-chord", Some("c2")),
            ("chords-menu", Some("m2")),
            ("chords-com.apple.finder", Some("f2")),
        ]);
        let plan = plan(&before, None, &current, &previous);
        assert!(plan.install.is_empty() && plan.remove.is_empty());
        assert_eq!(plan.offered, slugs(ALL));

        // Updated since, and Chord has saved what it offered.
        let after = installed(&[("chords-chord", Some("c3")), ("chords-menu", Some("m3"))]);
        assert!(
            super::plan(&after, Some(&plan.offered), &current, &previous)
                .install
                .is_empty()
        );
    }

    #[test]
    fn a_new_default_is_installed_once() {
        let (mut current, previous) = fixture();
        let offered = slugs(ALL);
        current.push(spec("chords-com.apple.Spotlight", "p1"));
        let plan = plan(
            &installed(&[("chords-chord", Some("c2"))]),
            Some(&offered),
            &current,
            &previous,
        );
        assert_eq!(names(&plan.install), ["chords-com.apple.Spotlight@p1"]);
        assert!(
            plan.offered
                .contains(&"keychord/chords-com.apple.spotlight".to_string())
        );
        // Removed after that, it isn't installed again.
        let again = super::plan(
            &installed(&[("chords-chord", Some("c2"))]),
            Some(&plan.offered),
            &current,
            &previous,
        );
        assert!(again.install.is_empty());
    }

    #[test]
    fn a_commit_shared_by_several_sets_matches_the_set_that_agrees_with_every_pin() {
        let current = vec![
            spec("chords-chord", "c3"),
            spec("chords-menu", "m3"),
            spec("chords-new", "n3"),
        ];
        let previous = vec![
            set(&[
                ("chords-chord", "c3"),
                ("chords-menu", "m2"),
                ("chords-new", "n2"),
            ]),
            set(&[("chords-chord", "c3"), ("chords-menu", "m1")]),
        ];
        let plan = plan(
            &installed(&[("chords-chord", Some("c3")), ("chords-menu", Some("m1"))]),
            None,
            &current,
            &previous,
        );
        assert_eq!(names(&plan.install), ["chords-new@n3"]);
    }

    #[test]
    fn an_install_without_a_matching_set_has_had_every_current_default() {
        let (current, previous) = fixture();
        let custom = installed(&[
            ("chords-chord", Some("c3")),
            ("chords-menu", None),
            ("chords-os", Some("o0")),
        ]);
        let plan = plan(&custom, None, &current, &previous);
        assert!(plan.install.is_empty() && plan.remove.is_empty());
        assert_eq!(plan.offered, slugs(ALL));
    }

    #[test]
    fn slugs_match_whatever_their_case() {
        let (current, previous) = fixture();
        let plan = plan(
            &[("keychord/CHORDS-OS".into(), Some("o1".into()))],
            Some(&["KeyChord/Chords-Chord".into()]),
            &current,
            &previous,
        );
        assert_eq!(plan.remove, ["keychord/CHORDS-OS"]);
        assert!(!names(&plan.install).contains(&"chords-chord@c3".to_string()));
    }
}
