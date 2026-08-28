use crate::api::{ApiImpl, AppError, AppResult};
use crate::app::AppHandleExt;
use crate::js_engine::JsEngine;

/// Persist the JS engine used for chord handlers (`"quickjs"` or `"bun"`).
/// The change applies after Chord restarts.
pub async fn set_js_engine(api: ApiImpl, engine: String) -> AppResult<()> {
    let handle = api.handle()?;
    let engine = JsEngine::parse(&engine)
        .ok_or_else(|| AppError::Message(format!("unknown JS engine `{engine}` (expected quickjs or bun)")))?;
    let settings = handle.app_state().settings();
    Ok(settings.set_js_engine(engine)?)
}
