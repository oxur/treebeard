//! Language frontend trait for Treebeard
//!
//! This module defines the `LanguageFrontend` trait that allows multiple languages
//! to target the Treebeard interpreter. Any language that can produce `syn` AST
//! can implement this trait and use Treebeard for immediate execution.
//!
//! # Architecture
//!
//! The frontend trait establishes a clean separation:
//!
//! ```text
//! Source Code → [Frontend] → syn AST → [Treebeard Core] → Value
//! ```
//!
//! Frontends are responsible for:
//! - Parsing source code into `syn` AST
//! - Macro expansion (if applicable)
//! - Language-specific error formatting
//! - Language-specific value formatting
//!
//! The interpreter core is responsible for:
//! - Evaluating `syn` AST nodes
//! - Managing runtime environment
//! - Ownership tracking (optional)

use crate::{EvalError, Value};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════

/// Error that occurred during parsing.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Human-readable error message
    pub message: String,

    /// Optional source location
    pub location: Option<SourceLocation>,

    /// Optional source snippet for context
    pub snippet: Option<String>,
}

impl ParseError {
    /// Create a new parse error with just a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            snippet: None,
        }
    }

    /// Add location information to the error.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Add a source snippet for context.
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error: {}", self.message)?;
        if let Some(loc) = &self.location {
            write!(f, " at {}:{}:{}", loc.file, loc.line, loc.column)?;
        }
        if let Some(snippet) = &self.snippet {
            write!(f, "\n{}", snippet)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Source code location for error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// File name or identifier
    pub file: String,

    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,
}

impl SourceLocation {
    /// Create a new source location.
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

/// Error that occurred during macro expansion.
#[derive(Debug, Clone)]
pub struct MacroError {
    /// Human-readable error message
    pub message: String,

    /// Macro name that caused the error
    pub macro_name: Option<String>,

    /// Optional source location
    pub location: Option<SourceLocation>,
}

impl MacroError {
    /// Create a new macro error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            macro_name: None,
            location: None,
        }
    }

    /// Add macro name to the error.
    pub fn with_macro_name(mut self, name: impl Into<String>) -> Self {
        self.macro_name = Some(name.into());
        self
    }

    /// Add location information to the error.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }
}

impl fmt::Display for MacroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Macro expansion error: {}", self.message)?;
        if let Some(name) = &self.macro_name {
            write!(f, " in macro `{}`", name)?;
        }
        if let Some(loc) = &self.location {
            write!(f, " at {}:{}:{}", loc.file, loc.line, loc.column)?;
        }
        Ok(())
    }
}

impl std::error::Error for MacroError {}

// ═══════════════════════════════════════════════════════════════════════
// MACRO ENVIRONMENT (PLACEHOLDER FOR PHASE 3)
// ═══════════════════════════════════════════════════════════════════════

/// Macro environment for compile-time macro expansion.
///
/// This is a placeholder for Phase 3: Macro System.
/// Currently empty, but will contain macro definitions and expansion state.
#[derive(Debug, Clone, Default)]
pub struct MacroEnvironment {
    /// Reserved for future macro storage
    _marker: std::marker::PhantomData<()>,
}

impl MacroEnvironment {
    /// Create a new empty macro environment.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// REPL COMMAND (PLACEHOLDER FOR PHASE 4)
// ═══════════════════════════════════════════════════════════════════════

/// REPL command metadata.
///
/// This is a placeholder for Phase 4: REPL Integration.
/// Frontends can define custom REPL commands.
#[derive(Debug, Clone)]
pub struct ReplCommand {
    /// Command name (without leading colon)
    pub name: String,

    /// Short description of what the command does
    pub description: String,

    /// Detailed help text
    pub help: String,
}

impl ReplCommand {
    /// Create a new REPL command.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            help: help.into(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// LANGUAGE FRONTEND TRAIT
// ═══════════════════════════════════════════════════════════════════════

/// Language frontend interface for Treebeard.
///
/// This trait defines the boundary between language-specific parsing/formatting
/// and the language-agnostic Treebeard interpreter core.
///
/// # Design Philosophy
///
/// The frontend is responsible for:
/// - **Parsing:** Convert source text to `syn` AST
/// - **Macro expansion:** Transform AST before evaluation (optional)
/// - **Error formatting:** Present errors in language-appropriate style
/// - **Value formatting:** Display values in language-appropriate syntax
///
/// The interpreter core handles:
/// - **Evaluation:** Walk the AST and compute results
/// - **Environment:** Manage variable and function bindings
/// - **Ownership:** Track value ownership at runtime (optional)
///
/// # Example Implementation
///
/// ```rust,ignore
/// use treebeard::frontend::{LanguageFrontend, ParseError, MacroError, MacroEnvironment};
/// use treebeard::{EvalError, Value};
///
/// struct RustFrontend;
///
/// impl LanguageFrontend for RustFrontend {
///     fn parse(&self, source: &str) -> Result<Vec<syn::Item>, ParseError> {
///         syn::parse_file(source)
///             .map(|file| file.items)
///             .map_err(|e| ParseError::new(format!("Syntax error: {}", e)))
///     }
///
///     fn expand_macros(
///         &self,
///         items: Vec<syn::Item>,
///         _env: &MacroEnvironment,
///     ) -> Result<(Vec<syn::Item>, MacroEnvironment), MacroError> {
///         // Rust macros are already expanded by syn
///         Ok((items, MacroEnvironment::new()))
///     }
///
///     fn format_error(&self, error: &EvalError, source: &str) -> String {
///         format!("error: {}", error)
///     }
///
///     fn format_value(&self, value: &Value, _depth: usize) -> String {
///         format!("{:?}", value)
///     }
///
///     fn name(&self) -> &str {
///         "Rust"
///     }
///
///     fn file_extension(&self) -> &str {
///         "rs"
///     }
/// }
/// ```
pub trait LanguageFrontend: Send + Sync {
    /// Parse source code into `syn` AST.
    ///
    /// This method converts source text into a sequence of top-level items
    /// (functions, constants, structs, etc.) that can be evaluated by Treebeard.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the source cannot be parsed.
    fn parse(&self, source: &str) -> Result<Vec<syn::Item>, ParseError>;

    /// Expand macros in the AST.
    ///
    /// This method transforms the AST by expanding macro invocations. The macro
    /// environment is threaded through to allow macros to define other macros.
    ///
    /// For languages without macros (or where macros are already expanded),
    /// this can simply return the input unchanged.
    ///
    /// # Phase 3 Note
    ///
    /// This is currently a placeholder. Full macro expansion will be implemented
    /// in Phase 3: Macro System.
    ///
    /// # Errors
    ///
    /// Returns `MacroError` if macro expansion fails.
    fn expand_macros(
        &self,
        items: Vec<syn::Item>,
        env: &MacroEnvironment,
    ) -> Result<(Vec<syn::Item>, MacroEnvironment), MacroError>;

    /// Format an evaluation error in a language-appropriate style.
    ///
    /// This method converts a Treebeard `EvalError` into a human-readable error
    /// message, potentially including source snippets, suggestions, and colorized
    /// output.
    ///
    /// # Parameters
    ///
    /// - `error`: The evaluation error to format
    /// - `source`: The original source code (for context and snippets)
    ///
    /// # Returns
    ///
    /// A formatted error message ready to display to the user.
    fn format_error(&self, error: &EvalError, source: &str) -> String;

    /// Format a value in a language-appropriate style.
    ///
    /// This method converts a Treebeard `Value` into a human-readable string
    /// using language-appropriate syntax. The depth parameter controls how
    /// deeply nested structures are displayed.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to format
    /// - `depth`: Maximum nesting depth (0 = compact, higher = more detail)
    ///
    /// # Returns
    ///
    /// A formatted string representation of the value.
    fn format_value(&self, value: &Value, depth: usize) -> String;

    /// Return the name of this language frontend.
    ///
    /// Examples: "Rust", "Oxur", "LFE-Rust"
    fn name(&self) -> &str;

    /// Return the file extension for this language.
    ///
    /// Examples: "rs", "oxr", "lfe"
    fn file_extension(&self) -> &str;

    /// Return language-specific REPL commands.
    ///
    /// This method allows frontends to define custom REPL commands
    /// (e.g., `:expand` for macro expansion, `:ast` to show AST).
    ///
    /// # Phase 4 Note
    ///
    /// This is currently a placeholder. REPL commands will be implemented
    /// in Phase 4: REPL Integration.
    ///
    /// # Returns
    ///
    /// A vector of REPL command definitions. Defaults to empty.
    fn repl_commands(&self) -> Vec<ReplCommand> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_creation() {
        let err = ParseError::new("unexpected token");
        assert_eq!(err.message, "unexpected token");
        assert!(err.location.is_none());
        assert!(err.snippet.is_none());
    }

    #[test]
    fn test_parse_error_with_location() {
        let err = ParseError::new("unexpected token")
            .with_location(SourceLocation::new("test.rs", 10, 5));
        assert_eq!(err.location.unwrap().line, 10);
    }

    #[test]
    fn test_parse_error_with_snippet() {
        let err = ParseError::new("unexpected token").with_snippet("let x = ;");
        assert_eq!(err.snippet.unwrap(), "let x = ;");
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new("test.rs", 42, 10);
        assert_eq!(loc.file, "test.rs");
        assert_eq!(loc.line, 42);
        assert_eq!(loc.column, 10);
    }

    #[test]
    fn test_macro_error_creation() {
        let err = MacroError::new("expansion failed");
        assert_eq!(err.message, "expansion failed");
        assert!(err.macro_name.is_none());
        assert!(err.location.is_none());
    }

    #[test]
    fn test_macro_error_with_macro_name() {
        let err = MacroError::new("expansion failed").with_macro_name("my_macro");
        assert_eq!(err.macro_name.unwrap(), "my_macro");
    }

    #[test]
    fn test_macro_environment_creation() {
        let env = MacroEnvironment::new();
        let _ = env; // Just ensure it compiles and can be created
    }

    #[test]
    fn test_repl_command_creation() {
        let cmd = ReplCommand::new("help", "Show help", "Display help information");
        assert_eq!(cmd.name, "help");
        assert_eq!(cmd.description, "Show help");
        assert_eq!(cmd.help, "Display help information");
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::new("unexpected token")
            .with_location(SourceLocation::new("test.rs", 10, 5));
        let display = format!("{}", err);
        assert!(display.contains("Parse error"));
        assert!(display.contains("unexpected token"));
        assert!(display.contains("test.rs:10:5"));
    }

    #[test]
    fn test_macro_error_display() {
        let err = MacroError::new("expansion failed")
            .with_macro_name("my_macro")
            .with_location(SourceLocation::new("test.oxr", 20, 3));
        let display = format!("{}", err);
        assert!(display.contains("Macro expansion error"));
        assert!(display.contains("expansion failed"));
        assert!(display.contains("my_macro"));
        assert!(display.contains("test.oxr:20:3"));
    }
}
