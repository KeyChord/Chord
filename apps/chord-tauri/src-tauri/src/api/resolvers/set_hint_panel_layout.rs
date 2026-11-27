use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;
use crate::models::ChordLayout;

/// Shows the Hint Panel as the sidebar or as the overlay along the bottom, as the layout
/// chords (`0ls`, `0lf`) do, from Settings.
pub async fn set_hint_panel_layout(api: ApiImpl, layout: String) -> AppResult<()> {
    let layout: ChordLayout = serde_json::from_value(serde_json::Value::String(layout))
        .map_err(|error| AppError::Message(format!("unknown Hint Panel layout: {error}")))?;
    let handle = api.handle()?;
    handle
        .app_state()
        .chord_mode_manager()
        .panel
        .set_layout(layout)?;
    Ok(())
}
