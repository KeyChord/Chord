use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Type)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        let full_detail = format!("{:?}", e);
        eprintln!("Error occurred: {}", full_detail);
        AppError::Message(e.to_string())
    }
}

impl AppError {
    /// Keeps the whole context chain, such as the line and column of a TOML parse error, for
    /// messages the user acts on.
    pub fn detailed(error: anyhow::Error) -> Self {
        AppError::Message(format!("{error:#}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
