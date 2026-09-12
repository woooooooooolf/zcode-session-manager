use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("ZCode is running — close it before destructive operations")]
    ZcodeRunning,
    #[error("limited mode: selected sessions cannot be deleted while ZCode is running")]
    Limited { violations: Vec<(String, &'static str)> },
    #[error("database format is not compatible")]
    Compat { problems: Vec<String> },
    #[error("database integrity check failed")]
    Corruption { details: Vec<String> },
    #[error("database not found: {0}")]
    DbNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
