//! Error types for Treebeard evaluation

use thiserror::Error;

/// Main error type for Treebeard operations
#[derive(Error, Debug)]
pub enum TreebeardError {
    /// Type mismatch error
    #[error("Type error: expected {expected}, got {got}")]
    TypeError {
        /// Expected type
        expected: String,
        /// Actual type received
        got: String,
    },

    /// Value error
    #[error("Value error: {0}")]
    ValueError(String),

    /// Feature not yet implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Result type alias for Treebeard operations
pub type Result<T> = std::result::Result<T, TreebeardError>;
