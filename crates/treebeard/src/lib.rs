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
//! See the [Treebeard Architecture Guide](../../crates/design/docs/architecture.md)
//! for detailed design documentation.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod evaluator;
pub mod value;
pub mod environment;
pub mod ownership;
pub mod error;

/// Treebeard version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
