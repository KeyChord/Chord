use crate::app::chord_package_manager::git::PinnedGitRepoSpec;
use crate::git::GitHubRepoRef;
use anyhow::Context;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct DefaultPackagesFile {
    packages: BTreeMap<String, DefaultPackageSource>,
}

#[derive(Deserialize)]
struct DefaultPackageSource {
    git: String,
    rev: String,
}

pub fn load_default_packages() -> anyhow::Result<Vec<PinnedGitRepoSpec>> {
    #[cfg(debug_assertions)]
    {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/default-packages.toml");
        if path.exists() {
            log::debug!("Loading default packages from disk: {}", path.display());
            let contents = std::fs::read_to_string(path)?;
            return parse_default_packages(&contents);
        }
    }

    parse_default_packages(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/default-packages.toml"
    )))
}

fn parse_default_packages(contents: &str) -> anyhow::Result<Vec<PinnedGitRepoSpec>> {
    let default_packages = toml::from_str::<DefaultPackagesFile>(contents)
        .context("Failed to parse the default packages")?;

    default_packages
        .packages
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
}
