use crate::api::{ApiImpl, AppResult};
use crate::app::chord_package_manager::{ChordsFileProblem, chords_file_problem};

/// Runs the loader's checks on unsaved editor contents.
pub async fn check_chords_file(_api: ApiImpl, contents: String) -> AppResult<Option<ChordsFileProblem>> {
    Ok(chords_file_problem(&contents))
}
