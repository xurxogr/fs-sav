//! Error types for fs-sav.

use thiserror::Error;

/// Errors that can occur during SAV parsing.
#[derive(Error, Debug)]
pub enum FsSavError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// UESave parsing error
    #[error("Failed to parse save file: {0}")]
    ParseError(String),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid save file structure
    #[error("Invalid save file structure: {0}")]
    InvalidStructure(String),

    /// Watch error
    #[error("Watch error: {0}")]
    WatchError(String),
}

/// Result type alias for fs-sav operations.
pub type Result<T> = std::result::Result<T, FsSavError>;
