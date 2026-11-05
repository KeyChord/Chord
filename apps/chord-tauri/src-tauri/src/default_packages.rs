use crate::app::chord_package_manager::git::PinnedGitRepoSpec;
use crate::git::GitHubRepoRef;
use anyhow::Context;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

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

/// What it takes to move an install from an earlier default set to the current one.
#[derive(Debug, Default)]
pub struct DefaultPackageUpgrade {
    /// Defaults to move to their current pin, and defaults to add.
    pub pin: Vec<PinnedGitRepoSpec>,
    /// Packages that are no longer defaults, still at the revision Chord installed.
    pub remove: Vec<String>,
}

impl DefaultPackageUpgrade {
    pub fn is_empty(&self) -> bool {
        self.pin.is_empty() && self.remove.is_empty()
    }
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

/// The upgrade for an install whose repositories are these `(slug, pinned revision)` pairs.
pub fn plan_default_package_upgrade(
    installed: &[(String, Option<String>)],
) -> anyhow::Result<DefaultPackageUpgrade> {
    let file = default_packages_file()?;
    let previous = file.previous.clone();
    Ok(plan_upgrade(installed, &pinned_specs(file)?, &previous))
}

fn plan_upgrade(
    installed: &[(String, Option<String>)],
    current: &[PinnedGitRepoSpec],
    previous: &[BTreeMap<String, String>],
) -> DefaultPackageUpgrade {
    let key = |slug: &str| slug.to_ascii_lowercase();
    let previous = previous
        .iter()
        .map(|set| {
            set.iter()
                .map(|(slug, rev)| (key(slug), rev.as_str()))
                .collect::<HashMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let was_default =
        |slug: &str, rev: &str| previous.iter().any(|set| set.get(&key(slug)) == Some(&rev));

    // The newest set this install still has a package from is the one it was set up with.
    let Some(generation) = previous.iter().find(|set| {
        installed
            .iter()
            .any(|(slug, rev)| rev.as_deref().is_some_and(|rev| set.get(&key(slug)) == Some(&rev)))
    }) else {
        return DefaultPackageUpgrade::default();
    };

    let current_by_slug = current
        .iter()
        .map(|spec| (key(&spec.repo_ref.slug()), spec))
        .collect::<BTreeMap<_, _>>();
    let installed_slugs = installed
        .iter()
        .map(|(slug, _)| key(slug))
        .collect::<HashSet<_>>();

    let mut upgrade = DefaultPackageUpgrade::default();
    for (slug, rev) in installed {
        // A package the user pinned elsewhere, or that follows its branch, stays theirs.
        let Some(rev) = rev.as_deref().filter(|rev| was_default(slug, rev)) else {
            continue;
        };
        match current_by_slug.get(&key(slug)) {
            Some(spec) if spec.rev != rev => upgrade.pin.push((*spec).clone()),
            Some(_) => {}
            None => upgrade.remove.push(slug.clone()),
        }
    }
    // A default the install's set didn't have can't have been removed by the user.
    for (slug, spec) in &current_by_slug {
        if !installed_slugs.contains(slug) && !generation.contains_key(slug) {
            upgrade.pin.push((*spec).clone());
        }
    }
    upgrade.pin.sort_by_key(|spec| key(&spec.repo_ref.slug()));
    upgrade.remove.sort();
    upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_default_packages_parse() {
        let packages = load_default_packages().unwrap();
        assert!(packages
            .iter()
            .any(|package| package.repo_ref.slug() == "KeyChord/chords-chord"));
    }

    #[test]
    fn no_current_pin_is_listed_as_previous() {
        let file = default_packages_file().unwrap();
        let previous = file
            .previous
            .iter()
            .flat_map(|set| set.values().cloned())
            .collect::<HashSet<_>>();
        assert!(!file.previous.is_empty());
        for spec in pinned_specs(file).unwrap() {
            assert!(!previous.contains(&spec.rev), "{} is current and previous", spec.repo_ref.slug());
        }
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
        specs.iter().map(|spec| format!("{}@{}", spec.repo_ref.name, spec.rev)).collect()
    }

    fn fixture() -> (Vec<PinnedGitRepoSpec>, Vec<BTreeMap<String, String>>) {
        let current = vec![
            spec("chords-chord", "c3"),
            spec("chords-menu", "m3"),
            spec("chords-com.apple.finder", "f3"),
            spec("chords-com.apple.Safari", "s3"),
        ];
        let previous = vec![
            set(&[("chords-chord", "c2"), ("chords-menu", "m2"), ("chords-com.apple.finder", "f2"), ("chords-com.apple.Safari", "s2")]),
            set(&[("chords-chord", "c1"), ("chords-menu", "m1"), ("chords-os", "o1")]),
        ];
        (current, previous)
    }

    #[test]
    fn an_older_install_moves_to_the_current_set() {
        let (current, previous) = fixture();
        let upgrade = plan_upgrade(
            &installed(&[("chords-chord", Some("c1")), ("chords-menu", Some("m1")), ("chords-os", Some("o1")), ("chords-terminal", None)]),
            &current,
            &previous,
        );
        assert_eq!(names(&upgrade.pin), ["chords-chord@c3", "chords-com.apple.finder@f3", "chords-com.apple.Safari@s3", "chords-menu@m3"]);
        assert_eq!(upgrade.remove, ["KeyChord/chords-os"]);
    }

    #[test]
    fn a_default_the_user_removed_stays_removed() {
        let (current, previous) = fixture();
        // Set up with the newer set, which had Safari, and the user removed it.
        let upgrade = plan_upgrade(
            &installed(&[("chords-chord", Some("c2")), ("chords-menu", Some("m2")), ("chords-com.apple.finder", Some("f2"))]),
            &current,
            &previous,
        );
        assert_eq!(names(&upgrade.pin), ["chords-chord@c3", "chords-com.apple.finder@f3", "chords-menu@m3"]);
        assert!(upgrade.remove.is_empty());
    }

    #[test]
    fn current_and_custom_pins_are_left_alone() {
        let (current, previous) = fixture();
        let up_to_date = installed(&[("chords-chord", Some("c3")), ("chords-menu", Some("m3")), ("chords-com.apple.finder", Some("f3")), ("chords-com.apple.Safari", Some("s3"))]);
        assert!(plan_upgrade(&up_to_date, &current, &previous).is_empty());
        let custom = installed(&[("chords-chord", Some("mine")), ("chords-menu", None), ("chords-os", Some("o0"))]);
        assert!(plan_upgrade(&custom, &current, &previous).is_empty());
    }

    #[test]
    fn slugs_match_whatever_their_case() {
        let (current, previous) = fixture();
        let upgrade = plan_upgrade(&[("keychord/CHORDS-OS".into(), Some("o1".into()))], &current, &previous);
        assert_eq!(upgrade.remove, ["keychord/CHORDS-OS"]);
    }
}
