//! Error types for Treebeard evaluation

use thiserror::Error;

/// Evaluation error
#[derive(Debug, Error)]
pub enum EvalError {
    /// Placeholder error variant
    #[error("evaluation error: {0}")]
    Generic(String),
}

/// Result type for evaluation
pub type Result<T> = std::result::Result<T, EvalError>;
