use crate::state::GitRepo;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GitHubRepoRef {
    pub owner: String,
    pub name: String,
}

impl GitHubRepoRef {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Repository cannot be empty");
        }

        let slug = trimmed
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .strip_prefix("https://github.com/")
            .or_else(|| trimmed.strip_prefix("http://github.com/"))
            .or_else(|| trimmed.strip_prefix("git@github.com:"))
            .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))
            .unwrap_or(trimmed)
            .trim_matches('/');

        let mut parts = slug.split('/');
        let owner = parts
            .next()
            .filter(|segment| !segment.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Repository must be in the form owner/name"))?;
        let name = parts
            .next()
            .filter(|segment| !segment.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Repository must be in the form owner/name"))?;

        if parts.next().is_some() {
            anyhow::bail!("Repository must be in the form owner/name");
        }

        if owner.contains(char::is_whitespace) || name.contains(char::is_whitespace) {
            anyhow::bail!("Repository owner and name cannot contain spaces");
        }

        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    pub fn url(&self) -> String {
        format!("https://github.com/{}", self.slug())
    }

    pub fn local_abspath(&self, repos_root: &Path, rev: Option<&str>) -> PathBuf {
        let base = repos_root.join(&self.owner).join(&self.name);
        match rev {
            Some(rev) => base.join(rev),
            None => base.join("HEAD"),
        }
    }

    pub fn into_repo(self, repos_root: &Path) -> GitRepo {
        let slug = self.slug();
        let url = self.url();
        let local_abspath = self.local_abspath(repos_root, None);
        let head_short_sha = repo_head_short_sha(&local_abspath);
        GitRepo {
            owner: self.owner,
            name: self.name,
            slug,
            url,
            local_abspath,
            head_short_sha,
            pinned_rev: None,
        }
    }

    pub fn into_pinned_repo(self, repos_root: &Path, rev: impl Into<String>) -> GitRepo {
        let rev_str = rev.into();
        let slug = self.slug();
        let url = self.url();
        let local_abspath = self.local_abspath(repos_root, Some(&rev_str));
        let head_short_sha = repo_head_short_sha(&local_abspath);
        GitRepo {
            owner: self.owner,
            name: self.name,
            slug,
            url,
            local_abspath,
            head_short_sha,
            pinned_rev: Some(rev_str),
        }
    }
}

fn repo_head_short_sha(repo_path: &Path) -> Option<String> {
    let repo = gix::open(repo_path).ok()?;
    let mut head = repo.head().ok()?;
    let head_id = head.try_peel_to_id().ok()??;
    Some(head_id.shorten_or_id().to_string())
}

pub fn clone_repo(repo_ref: &GitHubRepoRef, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }

    let mut clone = gix::prepare_clone(repo_ref.url(), destination)?;
    let (mut checkout, checkout_outcome) =
        clone.fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    log::debug!(
        "Checkout outcome for {}: {:?}",
        repo_ref.slug(),
        checkout_outcome
    );
    let (_repo, worktree_outcome) =
        checkout.main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    log::debug!(
        "Worktree outcome for {}: {:?}",
        repo_ref.slug(),
        worktree_outcome
    );

    Ok(())
}

pub fn refresh_repo(repo_ref: &GitHubRepoRef, destination: &Path) -> anyhow::Result<()> {
    let temp_destination = destination.with_extension("syncing");
    if temp_destination.exists() {
        fs::remove_dir_all(&temp_destination)?;
    }

    clone_repo(repo_ref, &temp_destination)?;
    fs::remove_dir_all(destination)?;
    fs::rename(temp_destination, destination)?;

    Ok(())
}

pub fn clone_repo_at_revision(
    repo_ref: &GitHubRepoRef,
    destination: &Path,
    rev: &str,
) -> anyhow::Result<()> {
    clone_repo(repo_ref, destination)?;
    checkout_repo_revision(repo_ref, destination, rev)?;
    Ok(())
}

fn checkout_repo_revision(
    repo_ref: &GitHubRepoRef,
    repo_path: &Path,
    rev: &str,
) -> anyhow::Result<()> {
    let trimmed_rev = rev.trim();
    anyhow::ensure!(!trimmed_rev.is_empty(), "Revision cannot be empty");

    let repo = gix::open(repo_path)
        .with_context(|| format!("Failed to open repo at {}", repo_path.display()))?;

    let mut head = repo.head()?;
    let head_id = head.try_peel_to_id().ok().flatten();

    if let Some(id) = head_id {
        let id_str = id.to_string();
        if id_str.starts_with(trimmed_rev) || id_str == trimmed_rev {
            return Ok(());
        }
    }

    if let Ok(oid) = gix::ObjectId::from_hex(trimmed_rev.as_bytes()) {
        drop(repo);
        let temp_path = repo_path.with_extension("checkout");
        if temp_path.exists() {
            fs::remove_dir_all(&temp_path)?;
        }
        fs::create_dir_all(&temp_path)?;

        let mut clone = gix::prepare_clone(repo_ref.url(), &temp_path)?;
        let (mut checkout, _) =
            clone.fetch_only(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
        // checkout.write_pending(oid, gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
        fs::remove_dir_all(repo_path)?;
        fs::rename(temp_path, repo_path)?;
        return Ok(());
    }

    Ok(())
}

pub fn update_or_clone_repo_at_revision(
    repo_ref: &GitHubRepoRef,
    destination: &Path,
    rev: &str,
) -> anyhow::Result<()> {
    let trimmed_rev = rev.trim();
    anyhow::ensure!(!trimmed_rev.is_empty(), "Revision cannot be empty");

    let repo_is_git = destination.exists() && destination.join(".git").exists();

    if repo_is_git {
        log::info!(
            "Repository {} exists at {}. Fetching origin and checking out revision {}.",
            repo_ref.slug(),
            destination.display(),
            trimmed_rev
        );
        checkout_repo_revision(repo_ref, destination, trimmed_rev)?;
    } else {
        log::info!(
            "Repository {} does not exist at {}. Cloning from {} at revision {}.",
            repo_ref.slug(),
            destination.display(),
            repo_ref.url(),
            trimmed_rev
        );
        clone_repo_at_revision(repo_ref, destination, trimmed_rev)?;
    }

    let repo = gix::open(destination)?;
    let mut head = repo.head()?;
    let head_id = head.try_peel_to_id().ok().flatten();

    if let Some(id) = head_id {
        let current_sha = id.to_string();
        anyhow::ensure!(
            current_sha.starts_with(trimmed_rev) || current_sha == trimmed_rev,
            "Verified HEAD commit {} does not match revision {}",
            current_sha,
            trimmed_rev
        );
    }

    Ok(())
}
