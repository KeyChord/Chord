use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;

/// Installs the listed updates of these repositories, then reloads the chords once.
pub async fn update_packages(api: ApiImpl, repos: Vec<String>) -> AppResult<()> {
    let handle = api.handle()?;
    let state = handle.app_state();
    let chord_package_manager = state.chord_package_manager();
    let store = &chord_package_manager.registry.git.git_repos_store;
    let mut failures = Vec::new();
    let mut updated = 0;
    for slug in &repos {
        match store.install_update(slug) {
            Ok(_) => updated += 1,
            Err(error) => {
                log::error!("Couldn't update {slug}: {error:#}");
                failures.push(format!("{slug}: {error:#}"));
            }
        }
    }
    if updated > 0 {
        chord_package_manager.reload_all().await?;
    }
    if !failures.is_empty() {
        return Err(anyhow::anyhow!("Couldn't update {}", failures.join("; ")).into());
    }
    Ok(())
}
