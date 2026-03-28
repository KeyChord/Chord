use crate::app::git_repos_store::PinnedGitRepoSpec;
use crate::git::GitHubRepoRef;
use anyhow::Context;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct ChordpackFile {
    packages: BTreeMap<String, ChordpackPackageSource>,
}

#[derive(Deserialize)]
struct ChordpackPackageSource {
    git: String,
    rev: String,
}

pub fn load_default_chordpack() -> anyhow::Result<Vec<PinnedGitRepoSpec>> {
    parse_chordpack(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/chordpack.toml"
    )))
}

fn parse_chordpack(contents: &str) -> anyhow::Result<Vec<PinnedGitRepoSpec>> {
    let chordpack =
        toml::from_str::<ChordpackFile>(contents).context("Failed to parse chordpack")?;

    chordpack
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
