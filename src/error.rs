// src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DlogError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database Error: {0}")]
    Sql(#[from] rusqlite::Error),

    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Editor exited with a non-zero status")]
    EditorError,

    #[error("Log ID {0} not found")]
    LogNotFound(i32),

    #[error("No changes detected in log content")]
    NoChangesMade,
}

pub type Result<T> = std::result::Result<T, DlogError>;
