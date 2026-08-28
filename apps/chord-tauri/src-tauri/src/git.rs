use crate::state::GitRepo;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

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

    pub fn local_abspath(&self, repos_root: &Path, rev: &str) -> PathBuf {
        repos_root.join(&self.owner).join(&self.name).join(rev)
    }

    pub fn into_repo_at_revision(self, repos_root: &Path, rev: impl Into<String>) -> GitRepo {
        self.into_repo_with_revision(repos_root, rev.into(), None)
    }

    pub fn into_pinned_repo(self, repos_root: &Path, rev: impl Into<String>) -> GitRepo {
        let rev = rev.into();
        self.into_repo_with_revision(repos_root, rev.clone(), Some(rev))
    }

    fn into_repo_with_revision(
        self,
        repos_root: &Path,
        resolved_rev: String,
        pinned_rev: Option<String>,
    ) -> GitRepo {
        let slug = self.slug();
        let url = self.url();
        let local_abspath = self.local_abspath(repos_root, &resolved_rev);
        let head_short_sha = repo_head_short_sha(&local_abspath);
        GitRepo {
            owner: self.owner,
            name: self.name,
            slug,
            url,
            local_abspath,
            head_short_sha,
            pinned_rev,
        }
    }
}

fn repo_head_short_sha(repo_path: &Path) -> Option<String> {
    let repo = gix::open(repo_path).ok()?;
    let mut head = repo.head().ok()?;
    let head_id = head.try_peel_to_id().ok()??;
    Some(head_id.shorten_or_id().to_string())
}

pub(crate) fn repo_head_sha(repo_path: &Path) -> Result<String> {
    let repo = gix::open(repo_path)
        .with_context(|| format!("Failed to open repository at {}", repo_path.display()))?;
    let mut head = repo.head()?;
    let head_id = head
        .try_peel_to_id()?
        .with_context(|| format!("Repository at {} has an unborn HEAD", repo_path.display()))?;
    Ok(head_id.to_string())
}

fn normalize_revision(rev: &str) -> Result<String> {
    let trimmed_rev = rev.trim();
    anyhow::ensure!(!trimmed_rev.is_empty(), "Revision cannot be empty");
    let object_id = gix::ObjectId::from_hex(trimmed_rev.as_bytes())
        .with_context(|| format!("Pinned revision {trimmed_rev} must be a full object ID"))?;
    Ok(object_id.to_string())
}

fn verify_repo_at_revision(repo_path: &Path, rev: &str) -> Result<()> {
    let actual_rev = repo_head_sha(repo_path)?;
    anyhow::ensure!(
        actual_rev == rev,
        "Repository at {} has HEAD {}, expected {}",
        repo_path.display(),
        actual_rev,
        rev
    );

    let repo = gix::open(repo_path)?;
    anyhow::ensure!(
        !repo.is_dirty()?,
        "Repository at {} has tracked worktree changes",
        repo_path.display()
    );
    Ok(())
}

fn clone_repo_url(url: &str, destination: &Path, revision: Option<&str>) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    anyhow::ensure!(
        !destination.exists(),
        "Clone destination already exists: {}",
        destination.display()
    );

    let mut clone = gix::prepare_clone(url, destination)?;
    if let Some(revision) = revision {
        clone = clone.with_revision(Some(revision.to_owned()))?;
    }
    let (mut checkout, checkout_outcome) =
        clone.fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    log::debug!("Checkout outcome for {url}: {checkout_outcome:?}");
    let (_repo, worktree_outcome) =
        checkout.main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    log::debug!("Worktree outcome for {url}: {worktree_outcome:?}");

    Ok(())
}

fn temporary_checkout_path(destination: &Path) -> PathBuf {
    destination.with_extension(format!("materializing-{}", Uuid::new_v4()))
}

fn quarantine_path(destination: &Path) -> PathBuf {
    destination.with_extension(format!("invalid-{}", Uuid::new_v4()))
}

fn install_materialized_checkout(
    temp_path: &Path,
    destination: &Path,
    rev: &str,
    repo_label: &str,
) -> Result<()> {
    if destination.exists() {
        if verify_repo_at_revision(destination, rev).is_ok() {
            fs::remove_dir_all(temp_path)?;
            return Ok(());
        }

        let quarantine_path = quarantine_path(destination);
        log::warn!(
            "Quarantining invalid cache entry for {} from {} to {}",
            repo_label,
            destination.display(),
            quarantine_path.display()
        );
        fs::rename(destination, quarantine_path)?;
    }

    fs::rename(temp_path, destination)?;
    verify_repo_at_revision(destination, rev)?;
    Ok(())
}

fn materialize_repo_url_at_revision(
    repo_label: &str,
    url: &str,
    destination: &Path,
    rev: &str,
) -> Result<()> {
    let rev = normalize_revision(rev)?;

    if destination.exists() {
        match verify_repo_at_revision(destination, &rev) {
            Ok(()) => {
                log::debug!(
                    "Reusing immutable cache entry for {} at {}",
                    repo_label,
                    destination.display()
                );
                return Ok(());
            }
            Err(error) => log::warn!(
                "Cache entry for {} at {} is invalid and will be quarantined after replacement is verified: {error:#}",
                repo_label,
                destination.display()
            ),
        }
    }

    let temp_path = temporary_checkout_path(destination);
    let materialize_result = (|| {
        clone_repo_url(url, &temp_path, Some(&rev))?;
        verify_repo_at_revision(&temp_path, &rev)?;
        install_materialized_checkout(&temp_path, destination, &rev, repo_label)
    })();

    if materialize_result.is_err() && temp_path.exists() {
        let _ = fs::remove_dir_all(&temp_path);
    }

    materialize_result
}

pub fn materialize_repo_at_revision(
    repo_ref: &GitHubRepoRef,
    destination: &Path,
    rev: &str,
) -> Result<()> {
    materialize_repo_url_at_revision(&repo_ref.slug(), &repo_ref.url(), destination, rev)
}

fn materialize_repo_url_head(repo_label: &str, url: &str, repo_base: &Path) -> Result<String> {
    fs::create_dir_all(&repo_base)?;
    let temp_path = repo_base.join(format!(".materializing-{}", Uuid::new_v4()));

    let materialize_result = (|| {
        clone_repo_url(url, &temp_path, None)?;
        let rev = repo_head_sha(&temp_path)?;
        verify_repo_at_revision(&temp_path, &rev)?;
        let destination = repo_base.join(&rev);
        install_materialized_checkout(&temp_path, &destination, &rev, repo_label)?;
        Ok(rev)
    })();

    if materialize_result.is_err() && temp_path.exists() {
        let _ = fs::remove_dir_all(&temp_path);
    }

    materialize_result
}

pub fn materialize_repo_head(repo_ref: &GitHubRepoRef, repos_root: &Path) -> Result<GitRepo> {
    let repo_base = repos_root.join(&repo_ref.owner).join(&repo_ref.name);
    let rev = materialize_repo_url_head(&repo_ref.slug(), &repo_ref.url(), &repo_base)?;
    Ok(repo_ref.clone().into_repo_at_revision(repos_root, rev))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    struct TestRepository {
        root: PathBuf,
        source: PathBuf,
    }

    impl TestRepository {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("chord-git-test-{}", Uuid::new_v4()));
            let source = root.join("source");
            fs::create_dir_all(&source).unwrap();

            run_git(&source, &["init", "--initial-branch=main"]);
            run_git(&source, &["config", "user.email", "chord@example.com"]);
            run_git(&source, &["config", "user.name", "Chord Test"]);

            Self { root, source }
        }

        fn commit(&self, contents: &str, message: &str) -> String {
            fs::write(self.source.join("package.json"), contents).unwrap();
            run_git(&self.source, &["add", "package.json"]);
            run_git(&self.source, &["commit", "--quiet", "--message", message]);
            run_git(&self.source, &["rev-parse", "HEAD"])
        }

        fn source_url(&self) -> String {
            self.source.to_string_lossy().into_owned()
        }
    }

    impl Drop for TestRepository {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn run_git(current_dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(current_dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    #[test]
    fn materializes_a_non_head_revision_in_its_sha_directory() {
        let fixture = TestRepository::new();
        let pinned_rev = fixture.commit("first", "first");
        fixture.commit("second", "second");
        let destination = fixture.root.join("cache").join(&pinned_rev);

        materialize_repo_url_at_revision(
            "test/repo",
            &fixture.source_url(),
            &destination,
            &pinned_rev,
        )
        .unwrap();

        assert_eq!(repo_head_sha(&destination).unwrap(), pinned_rev);
        assert_eq!(
            fs::read_to_string(destination.join("package.json")).unwrap(),
            "first"
        );
    }

    #[test]
    fn quarantines_an_invalid_sha_cache_entry_after_replacement_is_ready() {
        let fixture = TestRepository::new();
        let pinned_rev = fixture.commit("first", "first");
        let head_rev = fixture.commit("second", "second");
        let cache_root = fixture.root.join("cache");
        let destination = cache_root.join(&pinned_rev);

        clone_repo_url(&fixture.source_url(), &destination, Some(&head_rev)).unwrap();
        materialize_repo_url_at_revision(
            "test/repo",
            &fixture.source_url(),
            &destination,
            &pinned_rev,
        )
        .unwrap();

        assert_eq!(repo_head_sha(&destination).unwrap(), pinned_rev);
        let quarantined = fs::read_dir(cache_root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(&format!("{pinned_rev}.invalid-"))
            })
            .unwrap();
        assert_eq!(repo_head_sha(&quarantined).unwrap(), head_rev);
    }

    #[test]
    fn syncing_head_activates_a_new_sha_without_removing_the_old_one() {
        let fixture = TestRepository::new();
        let first_rev = fixture.commit("first", "first");
        let cache_root = fixture.root.join("cache");

        let first_active_rev =
            materialize_repo_url_head("test/repo", &fixture.source_url(), &cache_root).unwrap();
        let second_rev = fixture.commit("second", "second");
        let second_active_rev =
            materialize_repo_url_head("test/repo", &fixture.source_url(), &cache_root).unwrap();

        assert_eq!(first_active_rev, first_rev);
        assert_eq!(second_active_rev, second_rev);
        assert_eq!(
            repo_head_sha(&cache_root.join(first_rev)).unwrap(),
            first_active_rev
        );
        assert_eq!(
            repo_head_sha(&cache_root.join(second_rev)).unwrap(),
            second_active_rev
        );
    }
}
