use crate::app::chord_package_manager::chord_package_registry::git::PinnedGitRepoSpec;
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
    #[cfg(debug_assertions)]
    {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/chordpack.toml");
        if path.exists() {
            log::debug!("Loading chordpack from disk: {}", path.display());
            let contents = std::fs::read_to_string(path)?;
            return parse_chordpack(&contents);
        }
    }

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
