use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("config file not found: {0}")]
    ConfigNotFound(String),
    #[error("invalid header value: {0}")]
    InvalidHeader(String),
    #[error("invalid pay type: {0}, expected ALI or WE_CHAT")]
    InvalidPayType(String),
    #[error(transparent)]
    Http(#[from] wreq::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type AppResult<T> = Result<T, AppError>;
