//! # Treebeard
//!
//! A tree-walking interpreter for Rust's `syn` AST.
//!
//! Treebeard is a language-agnostic execution engine that interprets Rust's
//! `syn` AST directly, enabling immediate execution without compilation.
//! Any language that can produce `syn` AST can leverage Treebeard for
//! rapid iteration, REPL environments, and gradual migration to compiled code.
//!
//! ## Architecture
//!
//! - **Language Frontend**: Parse source code to `syn` AST
//! - **Treebeard Core**: Interpret `syn` AST with ownership tracking
//! - **REPL**: Interactive session management
//! - **Compilation Escape**: Hot path optimization via `rustc`
//!
//! ## Status
//!
//! 🚧 **Work in Progress** - Core architecture defined, implementation underway.
//!
//! See the design documentation for detailed architecture information.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod context;
pub mod environment;
pub mod error;
pub mod eval;
pub mod evaluator;
pub mod ownership;
pub mod value;

// Re-export main types
pub use context::EvalContext;
pub use environment::{Binding, BindingMode, Environment, ScopeGuard};
pub use error::{EnvironmentError, EvalError, Result, TreebeardError};
pub use eval::{eval_block, eval_expr, ControlFlow, Evaluate};
pub use value::{
    BuiltinFn, BuiltinFnPtr, ClosureValue, CompiledFn, EnumData, EnumValue, FunctionValue,
    HashableValue, StructValue, Value, ValueRef, ValueRefMut,
};

/// Treebeard version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_exists() {
        assert!(!VERSION.is_empty());
    }
}
