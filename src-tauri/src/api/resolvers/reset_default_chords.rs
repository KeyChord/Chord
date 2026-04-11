use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::chordpack::load_default_chordpack;

pub async fn reset_default_chords(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let store = &handle.app_state().chord_package_manager().registry.git.git_repos_store;
    let default_chordpack = load_default_chordpack()?;
    store.replace_with_pinned_repos(default_chordpack)?;

    let chord_pm = handle.app_state().chord_package_manager();
    chord_pm.reload_all().await?;
    Ok(())
}
