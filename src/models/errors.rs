use std::fmt;

#[derive(Debug)]
pub struct AppError(pub String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("HTTP error: {0}")]
    HttpError(u16),
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("Failed to send notification: {0}")]
    SendFailed(#[from] reqwest::Error),
}
