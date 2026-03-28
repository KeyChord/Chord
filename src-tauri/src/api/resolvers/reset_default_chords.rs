use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::chordpack::load_default_chordpack;

pub async fn reset_default_chords(api: ApiImpl) -> AppResult<()> {
    let handle = api.handle()?;
    let store = handle.app_git_repos_store();
    let default_chordpack = load_default_chordpack()?;
    store.replace_with_pinned_repos(default_chordpack)?;

    let chord_registry = handle.app_chord_registry();
    chord_registry.reload().await?;
    Ok(())
}
