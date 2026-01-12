# Stage 2.1: Frontend Trait Definition

**Phase:** 2 - Frontend Trait
**Stage:** 2.1
**Prerequisites:** Phase 1 Complete (Core Evaluator)
**Estimated effort:** 1-2 days

---

## Objective

Design and implement the `LanguageFrontend` trait that allows multiple syntaxes (Rust, Oxur, potentially others) to target the Treebeard interpreter. This establishes the abstraction boundary between parsing/syntax and evaluation.

---

## Overview

The `LanguageFrontend` trait defines the contract between a language's syntax and Treebeard's evaluator:

```
Source Code → [Frontend: parse] → syn AST → [Treebeard: evaluate] → Value
                    ↓
             [Frontend: expand_macros] (optional)
                    ↓
             [Frontend: format_error] (for display)
```

**Key insight:** All frontends produce `syn` AST. Treebeard only evaluates `syn` AST. This keeps the evaluator simple and allows any syntax that can be converted to Rust's AST.

---

## File Structure

```
treebeard/src/
├── lib.rs              # Add frontend exports
├── frontend/
│   ├── mod.rs          # ← New: LanguageFrontend trait
│   ├── error.rs        # ← New: Frontend error types
│   ├── source_map.rs   # ← New: Span mapping
│   └── repl_command.rs # ← New: REPL command extension
└── ...
```

---

## LanguageFrontend Trait

### src/frontend/mod.rs

```rust
pub mod error;
pub mod source_map;
pub mod repl_command;

pub use error::{FrontendError, ParseError, MacroError};
pub use source_map::{SourceMap, SourceLocation};
pub use repl_command::ReplCommand;

use crate::{Value, EvalError, Environment, EvalContext};
use std::collections::HashMap;

/// A language frontend that can parse source code into `syn` AST.
///
/// This trait defines the contract between a language's syntax and Treebeard's
/// evaluator. Implementors handle parsing, macro expansion, and error formatting,
/// while Treebeard handles evaluation.
///
/// # Example
///
/// ```ignore
/// // Rust frontend
/// let rust = RustFrontend::new();
/// let items = rust.parse("fn add(a: i64, b: i64) -> i64 { a + b }")?;
///
/// // Oxur frontend (same semantics, different syntax)
/// let oxur = OxurFrontend::new();
/// let items = oxur.parse("(defn add [a:i64 b:i64] -> i64 (+ a b))")?;
///
/// // Both produce equivalent syn::Item::Fn
/// ```
pub trait LanguageFrontend: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════
    // Required Methods
    // ═══════════════════════════════════════════════════════════════════

    /// Parse source code into a sequence of `syn` items.
    ///
    /// This is the core parsing method. The frontend must convert its
    /// native syntax into Rust's `syn` AST types.
    fn parse(&self, source: &str) -> Result<Vec<syn::Item>, ParseError>;

    /// Get the name of this language (e.g., "Rust", "Oxur").
    fn name(&self) -> &str;

    /// Get the file extension for this language (e.g., "rs", "oxur").
    fn file_extension(&self) -> &str;

    // ═══════════════════════════════════════════════════════════════════
    // Optional Methods (with defaults)
    // ═══════════════════════════════════════════════════════════════════

    /// Parse a single expression.
    ///
    /// Used for REPL input and expression evaluation. Default implementation
    /// wraps the expression in a function and extracts it.
    fn parse_expr(&self, source: &str) -> Result<syn::Expr, ParseError> {
        // Default: try to parse as an expression by wrapping
        let wrapped = format!("fn __expr__() {{ {} }}", source);
        let items = self.parse(&wrapped)?;

        // Extract the expression from the function body
        if let Some(syn::Item::Fn(func)) = items.first() {
            if let Some(syn::Stmt::Expr(expr, _)) = func.block.stmts.first() {
                return Ok(expr.clone());
            }
        }

        Err(ParseError::InvalidExpression {
            source: source.to_string(),
            message: "could not parse as expression".to_string(),
        })
    }

    /// Expand macros in the AST.
    ///
    /// For languages with macro systems (like Oxur), this transforms the AST
    /// before evaluation. Languages without macros return the items unchanged.
    fn expand_macros(
        &self,
        items: Vec<syn::Item>,
        _macro_env: &mut MacroEnvironment,
    ) -> Result<Vec<syn::Item>, MacroError> {
        // Default: no macro expansion
        Ok(items)
    }

    /// Check if this frontend supports macros.
    fn supports_macros(&self) -> bool {
        false
    }

    /// Format an evaluation error for display.
    ///
    /// The frontend can map `syn` spans back to original source positions
    /// and produce language-appropriate error messages.
    fn format_error(&self, error: &EvalError, source: &str) -> String {
        // Default: use standard error formatting
        format_error_default(error, source)
    }

    /// Format a value for display.
    ///
    /// Different frontends may want different value representations.
    /// Oxur uses S-expressions, Rust uses Debug formatting.
    fn format_value(&self, value: &Value) -> String {
        // Default: use Debug formatting
        format!("{:?}", value)
    }

    /// Pretty-print a value with indentation.
    fn format_value_pretty(&self, value: &Value, indent: usize) -> String {
        // Default: just use format_value
        let prefix = " ".repeat(indent);
        format!("{}{}", prefix, self.format_value(value))
    }

    /// Get the source map for span-to-location conversion.
    fn source_map(&self) -> Option<&SourceMap> {
        None
    }

    /// Get REPL commands specific to this frontend.
    fn repl_commands(&self) -> Vec<ReplCommand> {
        vec![]
    }

    /// Check if the given source is a complete input (for multi-line REPL).
    ///
    /// Returns `true` if the input is complete and can be parsed,
    /// `false` if more input is needed (e.g., unclosed brackets).
    fn is_complete_input(&self, source: &str) -> bool {
        // Default: try to parse and see if it succeeds
        self.parse(source).is_ok()
    }

    /// Get prompt string for REPL.
    fn prompt(&self) -> &str {
        "> "
    }

    /// Get continuation prompt for multi-line input.
    fn continuation_prompt(&self) -> &str {
        "... "
    }

    /// Perform syntax highlighting on source code (for REPL).
    ///
    /// Returns the source with ANSI escape codes for highlighting.
    fn highlight(&self, source: &str) -> String {
        // Default: no highlighting
        source.to_string()
    }

    /// Get completions for the given prefix.
    fn completions(&self, prefix: &str, env: &Environment) -> Vec<String> {
        // Default: complete from environment
        env.all_names()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// Create a fresh macro environment.
    fn new_macro_environment(&self) -> MacroEnvironment {
        MacroEnvironment::new()
    }
}

/// Macro environment for frontends that support macros.
///
/// This is separate from the runtime `Environment` because macros
/// operate at compile-time, not run-time.
#[derive(Debug, Clone, Default)]
pub struct MacroEnvironment {
    /// Macro definitions: name -> definition
    macros: HashMap<String, MacroDefinition>,

    /// Gensym counter for hygiene
    gensym_counter: u64,
}

/// A macro definition.
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    /// Macro name
    pub name: String,

    /// Parameter names
    pub params: Vec<MacroParam>,

    /// The macro body (language-specific representation)
    pub body: MacroBody,

    /// Source location for error reporting
    pub source_span: Option<proc_macro2::Span>,
}

/// A macro parameter.
#[derive(Debug, Clone)]
pub struct MacroParam {
    pub name: String,
    pub kind: MacroParamKind,
}

/// Kind of macro parameter.
#[derive(Debug, Clone)]
pub enum MacroParamKind {
    /// Single expression: `x`
    Single,
    /// Rest parameter (zero or more): `& rest`
    Rest,
    /// Optional parameter: `? opt`
    Optional,
}

/// Macro body representation.
///
/// This is intentionally opaque to allow different frontends to use
/// different macro body representations.
#[derive(Debug, Clone)]
pub enum MacroBody {
    /// Quasiquoted template (for Lisp-style macros)
    Template(syn::Expr),

    /// Procedural macro (Rust function that transforms AST)
    Procedural(String), // Name of the function to call

    /// Inline rules (pattern → template)
    Rules(Vec<MacroRule>),
}

/// A macro rule (pattern → template).
#[derive(Debug, Clone)]
pub struct MacroRule {
    pub pattern: syn::Pat,
    pub template: syn::Expr,
}

impl MacroEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a macro.
    pub fn define(&mut self, name: impl Into<String>, definition: MacroDefinition) {
        self.macros.insert(name.into(), definition);
    }

    /// Look up a macro by name.
    pub fn get(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    /// Check if a macro is defined.
    pub fn contains(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Generate a unique symbol for hygiene.
    pub fn gensym(&mut self, prefix: &str) -> String {
        let counter = self.gensym_counter;
        self.gensym_counter += 1;
        format!("{}__G{}", prefix, counter)
    }

    /// Get all macro names.
    pub fn macro_names(&self) -> impl Iterator<Item = &str> {
        self.macros.keys().map(|s| s.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Default Error Formatting
// ═══════════════════════════════════════════════════════════════════════

fn format_error_default(error: &EvalError, source: &str) -> String {
    let mut output = String::new();

    // Error type and message
    output.push_str(&format!("error: {}\n", error));

    // Try to get span information
    if let Some(span) = error.span() {
        // Get line/column from span
        let start = span.start();
        let line = start.line;
        let col = start.column;

        output.push_str(&format!("  --> <input>:{}:{}\n", line, col));

        // Show the source line if possible
        if let Some(source_line) = source.lines().nth(line.saturating_sub(1)) {
            output.push_str(&format!("   |\n"));
            output.push_str(&format!("{:3} | {}\n", line, source_line));
            output.push_str(&format!("   | {}^\n", " ".repeat(col)));
        }
    }

    output
}

// ═══════════════════════════════════════════════════════════════════════
// Frontend Registry
// ═══════════════════════════════════════════════════════════════════════

/// Registry of available frontends.
#[derive(Default)]
pub struct FrontendRegistry {
    frontends: HashMap<String, Box<dyn LanguageFrontend>>,
    by_extension: HashMap<String, String>,
}

impl FrontendRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a frontend.
    pub fn register(&mut self, frontend: Box<dyn LanguageFrontend>) {
        let name = frontend.name().to_string();
        let ext = frontend.file_extension().to_string();
        self.by_extension.insert(ext, name.clone());
        self.frontends.insert(name, frontend);
    }

    /// Get a frontend by name.
    pub fn get(&self, name: &str) -> Option<&dyn LanguageFrontend> {
        self.frontends.get(name).map(|f| f.as_ref())
    }

    /// Get a frontend by file extension.
    pub fn get_by_extension(&self, ext: &str) -> Option<&dyn LanguageFrontend> {
        self.by_extension
            .get(ext)
            .and_then(|name| self.get(name))
    }

    /// Get all registered frontend names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.frontends.keys().map(|s| s.as_str())
    }
}
```

---

## Frontend Errors

### src/frontend/error.rs

```rust
use thiserror::Error;
use proc_macro2::Span;

/// Errors that can occur in a language frontend.
#[derive(Error, Debug, Clone)]
pub enum FrontendError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Macro(#[from] MacroError),
}

/// Errors during parsing.
#[derive(Error, Debug, Clone)]
pub enum ParseError {
    /// Syntax error in source code.
    #[error("syntax error: {message}")]
    Syntax {
        message: String,
        line: usize,
        column: usize,
        source_snippet: Option<String>,
    },

    /// Failed to parse as expression.
    #[error("invalid expression: {message}")]
    InvalidExpression {
        source: String,
        message: String,
    },

    /// Unexpected end of input.
    #[error("unexpected end of input: {context}")]
    UnexpectedEof {
        context: String,
    },

    /// Unbalanced delimiters.
    #[error("unbalanced {delimiter}: opened at {open_line}:{open_col}")]
    UnbalancedDelimiter {
        delimiter: String,
        open_line: usize,
        open_col: usize,
    },

    /// Invalid token.
    #[error("invalid token `{token}` at {line}:{column}")]
    InvalidToken {
        token: String,
        line: usize,
        column: usize,
    },

    /// syn parse error wrapper.
    #[error("parse error: {0}")]
    Syn(String),
}

impl From<syn::Error> for ParseError {
    fn from(err: syn::Error) -> Self {
        ParseError::Syn(err.to_string())
    }
}

/// Errors during macro expansion.
#[derive(Error, Debug, Clone)]
pub enum MacroError {
    /// Undefined macro.
    #[error("undefined macro `{name}`")]
    UndefinedMacro {
        name: String,
        span: Option<Span>,
    },

    /// Wrong number of arguments to macro.
    #[error("macro `{name}` expects {expected} arguments, got {got}")]
    ArityMismatch {
        name: String,
        expected: String, // e.g., "2", "2 or more", "at least 1"
        got: usize,
        span: Option<Span>,
    },

    /// Macro expansion produced invalid syntax.
    #[error("macro `{name}` produced invalid syntax: {message}")]
    InvalidExpansion {
        name: String,
        message: String,
        span: Option<Span>,
    },

    /// Infinite macro expansion detected.
    #[error("infinite macro expansion detected: {chain}")]
    InfiniteExpansion {
        chain: String, // e.g., "foo -> bar -> foo"
    },

    /// Maximum expansion depth exceeded.
    #[error("maximum macro expansion depth ({max}) exceeded")]
    MaxDepthExceeded {
        max: usize,
    },

    /// Error during quasiquote expansion.
    #[error("quasiquote error: {message}")]
    QuasiquoteError {
        message: String,
        span: Option<Span>,
    },

    /// Unquote outside of quasiquote.
    #[error("unquote outside of quasiquote")]
    UnquoteOutsideQuasiquote {
        span: Option<Span>,
    },
}
```

---

## Source Map

### src/frontend/source_map.rs

```rust
use proc_macro2::Span;
use std::collections::HashMap;

/// Maps `syn` spans to original source locations.
///
/// This is essential for error reporting in frontends where the
/// `syn` AST was generated from a different syntax (like Oxur).
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    /// Span ID → Source location
    locations: HashMap<SpanKey, SourceLocation>,

    /// Source file contents (for snippets)
    source: String,

    /// File name (for error messages)
    file_name: String,
}

/// Key for looking up spans (spans aren't hashable directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SpanKey {
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

impl From<Span> for SpanKey {
    fn from(span: Span) -> Self {
        let start = span.start();
        let end = span.end();
        SpanKey {
            start_line: start.line,
            start_col: start.column,
            end_line: end.line,
            end_col: end.column,
        }
    }
}

/// A location in the original source code.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (0-indexed)
    pub column: usize,

    /// End line (for multi-line spans)
    pub end_line: usize,

    /// End column
    pub end_column: usize,

    /// Optional label for the location
    pub label: Option<String>,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            end_line: line,
            end_column: column,
            label: None,
        }
    }

    pub fn with_end(mut self, end_line: usize, end_column: usize) -> Self {
        self.end_line = end_line;
        self.end_column = end_column;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl SourceMap {
    pub fn new(source: impl Into<String>, file_name: impl Into<String>) -> Self {
        Self {
            locations: HashMap::new(),
            source: source.into(),
            file_name: file_name.into(),
        }
    }

    /// Record a mapping from a syn span to a source location.
    pub fn record(&mut self, span: Span, location: SourceLocation) {
        self.locations.insert(span.into(), location);
    }

    /// Look up the source location for a span.
    pub fn lookup(&self, span: Span) -> Option<&SourceLocation> {
        self.locations.get(&span.into())
    }

    /// Get a snippet of source code around a location.
    pub fn snippet(&self, location: &SourceLocation, context_lines: usize) -> String {
        let lines: Vec<&str> = self.source.lines().collect();
        let start = location.line.saturating_sub(context_lines + 1);
        let end = (location.end_line + context_lines).min(lines.len());

        let mut output = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_num = start + i + 1;
            let marker = if line_num >= location.line && line_num <= location.end_line {
                ">"
            } else {
                " "
            };
            output.push_str(&format!("{} {:4} | {}\n", marker, line_num, line));
        }

        // Add underline for the specific location
        if location.line == location.end_line {
            let padding = 8 + location.column; // "  NNNN | " = 8 chars
            let underline_len = location.end_column.saturating_sub(location.column).max(1);
            output.push_str(&format!(
                "{}{}",
                " ".repeat(padding),
                "^".repeat(underline_len)
            ));
            if let Some(label) = &location.label {
                output.push_str(&format!(" {}", label));
            }
            output.push('\n');
        }

        output
    }

    /// Get the file name.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Get the full source.
    pub fn source(&self) -> &str {
        &self.source
    }
}
```

---

## REPL Commands

### src/frontend/repl_command.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use std::sync::Arc;

/// A REPL command provided by a frontend.
///
/// Commands are prefixed with `:` in the REPL (e.g., `:help`, `:macroexpand`).
#[derive(Clone)]
pub struct ReplCommand {
    /// Command name (without the `:` prefix)
    pub name: String,

    /// Short description for help
    pub description: String,

    /// Detailed help text
    pub help: String,

    /// The command handler
    pub handler: Arc<dyn ReplCommandHandler>,
}

impl ReplCommand {
    pub fn new<H>(name: impl Into<String>, description: impl Into<String>, handler: H) -> Self
    where
        H: ReplCommandHandler + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            help: String::new(),
            handler: Arc::new(handler),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = help.into();
        self
    }
}

impl std::fmt::Debug for ReplCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplCommand")
            .field("name", &self.name)
            .field("description", &self.description)
            .finish()
    }
}

/// Handler for a REPL command.
pub trait ReplCommandHandler: Send + Sync {
    /// Execute the command with the given arguments.
    fn execute(
        &self,
        args: &str,
        env: &mut Environment,
        ctx: &EvalContext,
    ) -> Result<ReplCommandResult, EvalError>;
}

/// Result of executing a REPL command.
#[derive(Debug)]
pub enum ReplCommandResult {
    /// Command produced a value to display
    Value(Value),

    /// Command produced text output
    Text(String),

    /// Command completed with no output
    Ok,

    /// Command requests REPL exit
    Exit,

    /// Command requests REPL reset
    Reset,
}

/// Function-based command handler.
impl<F> ReplCommandHandler for F
where
    F: Fn(&str, &mut Environment, &EvalContext) -> Result<ReplCommandResult, EvalError>
        + Send
        + Sync,
{
    fn execute(
        &self,
        args: &str,
        env: &mut Environment,
        ctx: &EvalContext,
    ) -> Result<ReplCommandResult, EvalError> {
        self(args, env, ctx)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Standard Commands (available to all frontends)
// ═══════════════════════════════════════════════════════════════════════

/// Create standard REPL commands available to all frontends.
pub fn standard_commands() -> Vec<ReplCommand> {
    vec![
        ReplCommand::new("help", "Show help", cmd_help)
            .with_help("Usage: :help [command]\n\nShow help for all commands or a specific command."),

        ReplCommand::new("quit", "Exit the REPL", cmd_quit)
            .with_help("Usage: :quit\n\nExit the REPL session."),

        ReplCommand::new("exit", "Exit the REPL", cmd_quit)
            .with_help("Usage: :exit\n\nExit the REPL session (alias for :quit)."),

        ReplCommand::new("clear", "Clear the environment", cmd_clear)
            .with_help("Usage: :clear\n\nClear all bindings except prelude."),

        ReplCommand::new("env", "Show environment bindings", cmd_env)
            .with_help("Usage: :env [pattern]\n\nShow all bindings or those matching pattern."),

        ReplCommand::new("type", "Show type of expression", cmd_type)
            .with_help("Usage: :type <expr>\n\nShow the type of an expression without evaluating it."),
    ]
}

fn cmd_help(
    _args: &str,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    Ok(ReplCommandResult::Text(
        "Available commands:\n  :help    - Show this help\n  :quit    - Exit the REPL\n  :clear   - Clear environment\n  :env     - Show bindings\n  :type    - Show expression type".to_string()
    ))
}

fn cmd_quit(
    _args: &str,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    Ok(ReplCommandResult::Exit)
}

fn cmd_clear(
    _args: &str,
    env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    env.clear();
    Ok(ReplCommandResult::Text("Environment cleared.".to_string()))
}

fn cmd_env(
    args: &str,
    env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    let pattern = args.trim();
    let names: Vec<_> = env
        .all_names()
        .filter(|name| pattern.is_empty() || name.contains(pattern))
        .collect();

    if names.is_empty() {
        return Ok(ReplCommandResult::Text("No bindings found.".to_string()));
    }

    let mut output = String::new();
    for name in names {
        if let Some(value) = env.get(name) {
            output.push_str(&format!("{}: {:?}\n", name, value));
        }
    }
    Ok(ReplCommandResult::Text(output))
}

fn cmd_type(
    args: &str,
    env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    let name = args.trim();
    if name.is_empty() {
        return Ok(ReplCommandResult::Text("Usage: :type <name>".to_string()));
    }

    if let Some(value) = env.get(name) {
        Ok(ReplCommandResult::Text(format!(
            "{}: {}",
            name,
            crate::error::type_name(value)
        )))
    } else {
        Ok(ReplCommandResult::Text(format!("undefined: {}", name)))
    }
}
```

---

## Update lib.rs

### Add to src/lib.rs

```rust
pub mod frontend;

// Frontend exports
pub use frontend::{
    LanguageFrontend,
    FrontendRegistry,
    MacroEnvironment,
    MacroDefinition,
    MacroParam,
    MacroParamKind,
    MacroBody,
    MacroRule,
    FrontendError,
    ParseError,
    MacroError,
    SourceMap,
    SourceLocation,
    ReplCommand,
    ReplCommandHandler,
    ReplCommandResult,
};
```

---

## Test Cases

### tests/frontend_tests.rs

```rust
use treebeard_core::*;

// ═══════════════════════════════════════════════════════════════════════
// MacroEnvironment Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_macro_env_new() {
    let env = MacroEnvironment::new();
    assert!(!env.contains("anything"));
}

#[test]
fn test_macro_env_define() {
    let mut env = MacroEnvironment::new();

    let def = MacroDefinition {
        name: "when".to_string(),
        params: vec![
            MacroParam { name: "test".to_string(), kind: MacroParamKind::Single },
            MacroParam { name: "body".to_string(), kind: MacroParamKind::Rest },
        ],
        body: MacroBody::Template(syn::parse_str("if test { body }").unwrap()),
        source_span: None,
    };

    env.define("when", def);
    assert!(env.contains("when"));
    assert!(env.get("when").is_some());
}

#[test]
fn test_macro_env_gensym() {
    let mut env = MacroEnvironment::new();

    let sym1 = env.gensym("temp");
    let sym2 = env.gensym("temp");
    let sym3 = env.gensym("other");

    assert!(sym1.starts_with("temp__G"));
    assert!(sym2.starts_with("temp__G"));
    assert!(sym3.starts_with("other__G"));
    assert_ne!(sym1, sym2);
}

// ═══════════════════════════════════════════════════════════════════════
// SourceMap Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_source_map_new() {
    let map = SourceMap::new("let x = 1;", "test.rs");
    assert_eq!(map.file_name(), "test.rs");
    assert_eq!(map.source(), "let x = 1;");
}

#[test]
fn test_source_map_snippet() {
    let source = "line 1\nline 2\nline 3\nline 4\nline 5";
    let map = SourceMap::new(source, "test.rs");

    let loc = SourceLocation::new(3, 0).with_end(3, 6);
    let snippet = map.snippet(&loc, 1);

    assert!(snippet.contains("line 2"));
    assert!(snippet.contains("line 3"));
    assert!(snippet.contains("line 4"));
    assert!(snippet.contains("^"));
}

// ═══════════════════════════════════════════════════════════════════════
// FrontendRegistry Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_registry_empty() {
    let registry = FrontendRegistry::new();
    assert!(registry.get("Rust").is_none());
}

// ═══════════════════════════════════════════════════════════════════════
// ReplCommand Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_standard_commands() {
    let commands = frontend::repl_command::standard_commands();

    let names: Vec<_> = commands.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"help"));
    assert!(names.contains(&"quit"));
    assert!(names.contains(&"clear"));
    assert!(names.contains(&"env"));
}

#[test]
fn test_command_quit() {
    let mut env = Environment::new();
    let ctx = EvalContext::default();

    let commands = frontend::repl_command::standard_commands();
    let quit = commands.iter().find(|c| c.name == "quit").unwrap();

    let result = quit.handler.execute("", &mut env, &ctx).unwrap();
    assert!(matches!(result, ReplCommandResult::Exit));
}

// ═══════════════════════════════════════════════════════════════════════
// Error Formatting Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_error_display() {
    let err = ParseError::Syntax {
        message: "unexpected token".to_string(),
        line: 1,
        column: 5,
        source_snippet: Some("let x = @;".to_string()),
    };

    let msg = err.to_string();
    assert!(msg.contains("syntax error"));
    assert!(msg.contains("unexpected token"));
}

#[test]
fn test_macro_error_display() {
    let err = MacroError::UndefinedMacro {
        name: "foobar".to_string(),
        span: None,
    };

    let msg = err.to_string();
    assert!(msg.contains("undefined macro"));
    assert!(msg.contains("foobar"));
}
```

---

## Completion Checklist

- [ ] Create `src/frontend/mod.rs` with `LanguageFrontend` trait
- [ ] Implement all required methods in trait
- [ ] Implement default methods (parse_expr, format_error, etc.)
- [ ] Create `MacroEnvironment` for macro-supporting frontends
- [ ] Create `MacroDefinition`, `MacroParam`, `MacroBody`, `MacroRule` types
- [ ] Create `src/frontend/error.rs` with `ParseError` and `MacroError`
- [ ] Create `src/frontend/source_map.rs` with `SourceMap` and `SourceLocation`
- [ ] Create `src/frontend/repl_command.rs` with `ReplCommand` infrastructure
- [ ] Implement standard REPL commands (:help, :quit, :clear, :env, :type)
- [ ] Create `FrontendRegistry` for managing multiple frontends
- [ ] Update `lib.rs` with frontend exports
- [ ] All tests passing

---

## Design Notes

### Why a Trait?

Using a trait allows:

- Multiple syntaxes targeting the same evaluator
- Clean separation of concerns (parsing vs evaluation)
- Easy testing (mock frontends)
- Future extensibility (add new languages)

### Why Produce `syn` AST?

`syn` is the standard Rust AST representation. By standardizing on it:

- Treebeard only needs one evaluator
- Frontends can leverage `syn`'s ecosystem (quote, proc-macro2)
- Direct path to compilation (syn → quote → rustc)

### Why Separate MacroEnvironment?

Macros are compile-time, not run-time. Keeping them separate:

- Prevents confusion between macro and runtime definitions
- Enables macro expansion before evaluation
- Matches how real compilers work

### Why Include SourceMap?

For frontends like Oxur, the `syn` spans don't match the original source. SourceMap enables mapping errors back to the user's actual code.

---

## Next Stage

**Stage 2.2: Rust Frontend** — Implement a trivial frontend that parses Rust source using `syn::parse_str` and `syn::parse_file`.
