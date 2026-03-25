use crate::api::{ApiImpl, AppError, AppResult};
use crate::observables::get_all_observable_states;

pub async fn get_current_states(api: ApiImpl) -> AppResult<String> {
    let handle = api.handle()?;
    let states = get_all_observable_states(handle.into())?;
    Ok(serde_json::to_string(&states).map_err(|_err| AppError::Message("what".into()))?)
}
