//! Error types for Treebeard evaluation

use proc_macro2::Span;
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

/// Errors that can occur during environment operations
#[derive(Error, Debug, Clone)]
pub enum EnvironmentError {
    /// Attempted to access an undefined variable
    #[error("undefined variable `{name}`")]
    UndefinedVariable {
        /// Variable name
        name: String,
    },

    /// Attempted to mutate an immutable binding
    #[error("cannot assign to immutable binding `{name}`")]
    ImmutableBinding {
        /// Binding name
        name: String,
        /// Location where binding was defined
        span: Option<Span>,
    },

    /// Call stack overflow (too much recursion)
    #[error("stack overflow: call depth {depth} exceeds maximum {max}")]
    StackOverflow {
        /// Current call depth
        depth: usize,
        /// Maximum allowed depth
        max: usize,
    },

    /// Attempted to redefine a constant
    #[error("cannot redefine constant `{name}`")]
    ConstantRedefinition {
        /// Constant name
        name: String,
    },
}
