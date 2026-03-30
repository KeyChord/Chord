use crate::observables::GitRepo;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    pub fn local_path(&self, repos_root: &Path) -> PathBuf {
        repos_root.join(&self.owner).join(&self.name)
    }

    pub fn into_repo(self, repos_root: &Path) -> GitRepo {
        let slug = self.slug();
        let url = self.url();
        let local_path = self.local_path(repos_root);
        let head_short_sha = repo_head_short_sha(&local_path);
        GitRepo {
            owner: self.owner,
            name: self.name,
            slug,
            url,
            local_path,
            head_short_sha,
            pinned_rev: None,
        }
    }

    pub fn into_pinned_repo(self, repos_root: &Path, rev: impl Into<String>) -> GitRepo {
        let mut repo = self.into_repo(repos_root);
        repo.pinned_rev = Some(rev.into());
        repo
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
    checkout_repo_revision(destination, rev)?;
    Ok(())
}

fn checkout_repo_revision(repo_path: &Path, rev: &str) -> anyhow::Result<()> {
    let trimmed_rev = rev.trim();
    anyhow::ensure!(!trimmed_rev.is_empty(), "Revision cannot be empty");

    let checkout_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("checkout")
        .arg("--detach")
        .arg(trimmed_rev)
        .output()
        .with_context(|| {
            format!(
                "Failed to run git checkout for revision {trimmed_rev} in {}",
                repo_path.display()
            )
        })?;

    if checkout_output.status.success() {
        // Successfully checked out directly
    } else {
        let fetch_output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("fetch")
            .arg("--depth")
            .arg("1")
            .arg("origin")
            .arg(trimmed_rev)
            .output()
            .with_context(|| {
                format!(
                    "Failed to fetch revision {trimmed_rev} for {}",
                    repo_path.display()
                )
            })?;

        anyhow::ensure!(
            fetch_output.status.success(),
            "Failed to fetch revision {trimmed_rev} for {}: {}",
            repo_path.display(),
            String::from_utf8_lossy(&fetch_output.stderr).trim()
        );

        let final_checkout_output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("checkout")
            .arg("--detach")
            .arg("FETCH_HEAD")
            .output()
            .with_context(|| {
                format!(
                    "Failed to check out fetched revision {trimmed_rev} in {}",
                    repo_path.display()
                )
            })?;

        anyhow::ensure!(
            final_checkout_output.status.success(),
            "Failed to check out revision {trimmed_rev} in {}: {}",
            repo_path.display(),
            String::from_utf8_lossy(&final_checkout_output.stderr).trim()
        );

        // --- Added verification step ---
        let current_head_output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .with_context(|| {
                format!(
                    "Failed to run git rev-parse HEAD for verification in {}",
                    repo_path.display()
                )
            })?;

        anyhow::ensure!(
            current_head_output.status.success(),
            "Failed to get current HEAD for verification in {}: {}",
            repo_path.display(),
            String::from_utf8_lossy(&current_head_output.stderr).trim()
        );

        let current_sha = String::from_utf8_lossy(&current_head_output.stdout).trim().to_string();
        // Ensure the rev from chordpack.toml is also trimmed for comparison
        let target_rev_trimmed = trimmed_rev.trim();

        anyhow::ensure!(
            current_sha == target_rev_trimmed,
            "Verified HEAD commit {} does not match target revision {} for repository at {}",
            current_sha,
            target_rev_trimmed,
            repo_path.display()
        );
        // --- End of added verification step ---
    }

    Ok(())
}

// --- NEW FUNCTION TO ADD ---

/// Updates an existing repository by fetching origin and checking out a revision,
/// or clones a new repository if it doesn't exist.
pub fn update_or_clone_repo_at_revision(
    repo_ref: &GitHubRepoRef,
    destination: &Path,
    rev: &str,
) -> anyhow::Result<()> {
    let trimmed_rev = rev.trim();
    anyhow::ensure!(!trimmed_rev.is_empty(), "Revision cannot be empty");

    // Check if the repository directory exists and is a git repository.
    let repo_is_git = destination.exists() && destination.join(".git").exists();

    if repo_is_git {
        log::info!("Repository {} exists at {}. Fetching origin and checking out revision {}.", repo_ref.slug(), destination.display(), trimmed_rev);
        
        // Fetch from origin
        let fetch_output = Command::new("git")
            .current_dir(destination) // Run command in the repo's directory
            .arg("fetch")
            .arg("origin")
            .output()
            .with_context(|| format!("Failed to fetch origin for {}", destination.display()))?;

        anyhow::ensure!(
            fetch_output.status.success(),
            "Failed to fetch origin for {}: {}",
            destination.display(),
            String::from_utf8_lossy(&fetch_output.stderr).trim()
        );

        // Checkout the specific revision
        let checkout_output = Command::new("git")
            .current_dir(destination) // Ensure command is run in the repo's directory
            .arg("checkout")
            .arg("--detach")
            .arg(trimmed_rev)
            .output()
            .with_context(|| {
                format!(
                    "Failed to run git checkout for revision {trimmed_rev} in {}",
                    destination.display()
                )
            })?;

        anyhow::ensure!(
            checkout_output.status.success(),
            "Failed to checkout revision {trimmed_rev} in {}: {}",
            destination.display(),
            String::from_utf8_lossy(&checkout_output.stderr).trim()
        );
    } else {
        log::info!("Repository {} does not exist at {}. Cloning from {} at revision {}.", repo_ref.slug(), destination.display(), repo_ref.url(), trimmed_rev);
        // If it does not exist, clone it and then checkout the revision
        // `clone_repo_at_revision` calls `clone_repo` and then `checkout_repo_revision`.
        // The `checkout_repo_revision` part handles checking out the specific revision.
        clone_repo_at_revision(repo_ref, destination, trimmed_rev)?;
    }

    // --- Verification step ---
    // This verification step ensures the correct revision is checked out.
    let current_head_output = Command::new("git")
        .current_dir(destination)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .with_context(|| {
            format!(
                "Failed to run git rev-parse HEAD for verification in {}",
                destination.display()
            )
        })?;

    anyhow::ensure!(
        current_head_output.status.success(),
        "Failed to get current HEAD for verification in {}: {}",
        destination.display(),
        String::from_utf8_lossy(&current_head_output.stderr).trim()
    );

    let current_sha = String::from_utf8_lossy(&current_head_output.stdout).trim().to_string();
    
    // Resolve the input revision string to a commit SHA for robust comparison.
    let git_rev_parse_output = Command::new("git")
        .current_dir(destination)
        .arg("rev-parse")
        .arg(trimmed_rev) // Use the trimmed input revision string
        .output()
        .with_context(|| format!("Failed to rev-parse revision {} in {}", trimmed_rev, destination.display()))?;

    let resolved_sha = if git_rev_parse_output.status.success() {
        String::from_utf8_lossy(&git_rev_parse_output.stdout).trim().to_string()
    } else {
        log::warn!("Could not resolve revision {} for {}. Verification will rely on HEAD commit match.", trimmed_rev, destination.display());
        String::new() // Indicates no resolvable SHA from the input revision string
    };

    // Ensure the current HEAD commit matches the resolved revision.
    anyhow::ensure!(
        resolved_sha.is_empty() || current_sha == resolved_sha,
        "Verified HEAD commit {} does not match the resolved revision {} for repository at {}",
        current_sha,
        resolved_sha,
        destination.display()
    );

    Ok(())
}
