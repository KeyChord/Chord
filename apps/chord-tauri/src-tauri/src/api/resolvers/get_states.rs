use crate::api::{ApiImpl, AppError, AppResult};
use crate::state::get_observable_states;

pub async fn get_states(_api: ApiImpl, ids: Vec<String>) -> AppResult<String> {
    let states = get_observable_states(&ids)?;
    serde_json::to_string(&states).map_err(|error| AppError::Message(error.to_string()))
}
