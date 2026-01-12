# Stage 2.2: Rust Frontend

**Phase:** 2 - Frontend Trait  
**Stage:** 2.2  
**Prerequisites:** Stage 2.1 (Frontend Trait Definition)  
**Estimated effort:** 0.5-1 day

---

## Objective

Implement a `RustFrontend` that parses Rust source code using `syn::parse_str` and `syn::parse_file`. This is the simplest possible frontend — essentially a thin wrapper around `syn` — but it validates the trait design and provides a baseline for testing.

---

## Overview

The Rust frontend is trivial because `syn` already parses Rust:

```
Rust Source → syn::parse_str → syn AST → Treebeard
```

No transformation needed. This frontend exists to:
1. Validate the `LanguageFrontend` trait design
2. Enable pure-Rust testing of Treebeard
3. Serve as reference implementation for other frontends

---

## File Structure

```
treebeard-core/src/
├── frontend/
│   ├── mod.rs          # Add rust module
│   ├── rust.rs         # ← New: RustFrontend
│   └── ...
└── ...
```

---

## RustFrontend Implementation

### src/frontend/rust.rs

```rust
use super::{
    LanguageFrontend, ParseError, MacroError, MacroEnvironment,
    SourceMap, SourceLocation, ReplCommand, ReplCommandResult,
};
use crate::{Value, Environment, EvalContext, EvalError};

/// Rust language frontend.
///
/// This is the simplest possible frontend — it just delegates to `syn`.
/// It serves as:
/// - Validation of the `LanguageFrontend` trait design
/// - Baseline for testing Treebeard with native Rust syntax
/// - Reference implementation for other frontends
#[derive(Debug, Default)]
pub struct RustFrontend {
    /// Source map for error reporting (populated during parsing)
    source_map: Option<SourceMap>,
}

impl RustFrontend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LanguageFrontend for RustFrontend {
    // ═══════════════════════════════════════════════════════════════════
    // Required Methods
    // ═══════════════════════════════════════════════════════════════════

    fn parse(&self, source: &str) -> Result<Vec<syn::Item>, ParseError> {
        let file: syn::File = syn::parse_str(source)?;
        Ok(file.items)
    }

    fn name(&self) -> &str {
        "Rust"
    }

    fn file_extension(&self) -> &str {
        "rs"
    }

    // ═══════════════════════════════════════════════════════════════════
    // Overridden Methods
    // ═══════════════════════════════════════════════════════════════════

    fn parse_expr(&self, source: &str) -> Result<syn::Expr, ParseError> {
        // syn can parse expressions directly
        syn::parse_str(source).map_err(ParseError::from)
    }

    fn format_value(&self, value: &Value) -> String {
        // Rust-style formatting
        match value {
            Value::Unit => "()".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Char(c) => format!("'{}'", c.escape_default()),
            Value::String(s) => format!("{:?}", s.as_str()),
            Value::I8(n) => format!("{}i8", n),
            Value::I16(n) => format!("{}i16", n),
            Value::I32(n) => format!("{}i32", n),
            Value::I64(n) => format!("{}", n), // Default integer type
            Value::I128(n) => format!("{}i128", n),
            Value::Isize(n) => format!("{}isize", n),
            Value::U8(n) => format!("{}u8", n),
            Value::U16(n) => format!("{}u16", n),
            Value::U32(n) => format!("{}u32", n),
            Value::U64(n) => format!("{}u64", n),
            Value::U128(n) => format!("{}u128", n),
            Value::Usize(n) => format!("{}usize", n),
            Value::F32(n) => format!("{}f32", n),
            Value::F64(n) => {
                if n.fract() == 0.0 {
                    format!("{}.0", n)
                } else {
                    format!("{}", n)
                }
            }
            Value::Tuple(elements) => {
                if elements.is_empty() {
                    "()".to_string()
                } else {
                    let inner: Vec<_> = elements.iter().map(|v| self.format_value(v)).collect();
                    format!("({})", inner.join(", "))
                }
            }
            Value::Array(elements) => {
                let inner: Vec<_> = elements.iter().map(|v| self.format_value(v)).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Vec(elements) => {
                let inner: Vec<_> = elements.iter().map(|v| self.format_value(v)).collect();
                format!("vec![{}]", inner.join(", "))
            }
            Value::Struct(s) => {
                let fields: Vec<_> = s
                    .fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.format_value(v)))
                    .collect();
                format!("{} {{ {} }}", s.type_name, fields.join(", "))
            }
            Value::Enum(e) => {
                match &e.data {
                    crate::EnumData::Unit => format!("{}::{}", e.type_name, e.variant),
                    crate::EnumData::Tuple(elements) => {
                        let inner: Vec<_> =
                            elements.iter().map(|v| self.format_value(v)).collect();
                        format!("{}::{}({})", e.type_name, e.variant, inner.join(", "))
                    }
                    crate::EnumData::Struct(fields) => {
                        let inner: Vec<_> = fields
                            .iter()
                            .map(|(k, v)| format!("{}: {}", k, self.format_value(v)))
                            .collect();
                        format!("{}::{} {{ {} }}", e.type_name, e.variant, inner.join(", "))
                    }
                }
            }
            Value::Option(opt) => match opt {
                Some(v) => format!("Some({})", self.format_value(v)),
                None => "None".to_string(),
            },
            Value::Result(res) => match res.as_ref() {
                Ok(v) => format!("Ok({})", self.format_value(v)),
                Err(e) => format!("Err({})", self.format_value(e)),
            },
            Value::Function(f) => format!("fn {}(...)", f.name),
            Value::Closure(_) => "closure".to_string(),
            Value::BuiltinFn(f) => format!("builtin::{}", f.name),
            Value::CompiledFn(f) => format!("compiled::{}", f.name),
            Value::HashMap(map) => {
                let inner: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("{} => {}", self.format_value(k), self.format_value(v)))
                    .collect();
                format!("{{ {} }}", inner.join(", "))
            }
            Value::Bytes(bytes) => {
                if bytes.len() <= 8 {
                    format!("{:?}", bytes.as_slice())
                } else {
                    format!("[{} bytes]", bytes.len())
                }
            }
            Value::Ref(r) => format!("&{}", self.format_value(&r.read())),
            Value::RefMut(r) => format!("&mut {}", self.format_value(&r.read())),
        }
    }

    fn format_value_pretty(&self, value: &Value, indent: usize) -> String {
        let prefix = "  ".repeat(indent);
        match value {
            Value::Struct(s) => {
                let mut output = format!("{} {{\n", s.type_name);
                for (k, v) in &s.fields {
                    output.push_str(&format!(
                        "{}{}: {},\n",
                        "  ".repeat(indent + 1),
                        k,
                        self.format_value_pretty(v, indent + 1).trim_start()
                    ));
                }
                output.push_str(&format!("{}}}", prefix));
                output
            }
            Value::Vec(elements) | Value::Array(elements) if elements.len() > 3 => {
                let bracket = if matches!(value, Value::Vec(_)) {
                    ("vec![", "]")
                } else {
                    ("[", "]")
                };
                let mut output = format!("{}\n", bracket.0);
                for v in elements.iter() {
                    output.push_str(&format!(
                        "{}{},\n",
                        "  ".repeat(indent + 1),
                        self.format_value(v)
                    ));
                }
                output.push_str(&format!("{}{}", prefix, bracket.1));
                output
            }
            _ => format!("{}{}", prefix, self.format_value(value)),
        }
    }

    fn is_complete_input(&self, source: &str) -> bool {
        // Try parsing as items
        if syn::parse_str::<syn::File>(source).is_ok() {
            return true;
        }

        // Try parsing as expression
        if syn::parse_str::<syn::Expr>(source).is_ok() {
            return true;
        }

        // Check for obviously incomplete input
        let trimmed = source.trim();
        
        // Empty input is complete (no-op)
        if trimmed.is_empty() {
            return true;
        }

        // Count brackets
        let mut brace_depth = 0i32;
        let mut paren_depth = 0i32;
        let mut bracket_depth = 0i32;
        let mut in_string = false;
        let mut in_char = false;
        let mut escape_next = false;

        for c in trimmed.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '\\' if in_string || in_char => escape_next = true,
                '"' if !in_char => in_string = !in_string,
                '\'' if !in_string => in_char = !in_char,
                '{' if !in_string && !in_char => brace_depth += 1,
                '}' if !in_string && !in_char => brace_depth -= 1,
                '(' if !in_string && !in_char => paren_depth += 1,
                ')' if !in_string && !in_char => paren_depth -= 1,
                '[' if !in_string && !in_char => bracket_depth += 1,
                ']' if !in_string && !in_char => bracket_depth -= 1,
                _ => {}
            }
        }

        // Incomplete if any brackets are unclosed
        if brace_depth > 0 || paren_depth > 0 || bracket_depth > 0 {
            return false;
        }

        // Incomplete if in string or char literal
        if in_string || in_char {
            return false;
        }

        // Otherwise assume complete (let syn report errors)
        true
    }

    fn prompt(&self) -> &str {
        "rust> "
    }

    fn continuation_prompt(&self) -> &str {
        "....| "
    }

    fn highlight(&self, source: &str) -> String {
        // Basic syntax highlighting using ANSI codes
        highlight_rust(source)
    }

    fn completions(&self, prefix: &str, env: &Environment) -> Vec<String> {
        let mut completions: Vec<String> = env
            .all_names()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect();

        // Add Rust keywords
        let keywords = [
            "let", "mut", "fn", "if", "else", "match", "loop", "while", "for",
            "in", "break", "continue", "return", "struct", "enum", "impl",
            "trait", "type", "const", "static", "pub", "use", "mod", "self",
            "true", "false", "as", "ref", "where",
        ];

        for kw in keywords {
            if kw.starts_with(prefix) && !completions.contains(&kw.to_string()) {
                completions.push(kw.to_string());
            }
        }

        // Add common types
        let types = [
            "i8", "i16", "i32", "i64", "i128", "isize",
            "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64", "bool", "char", "str", "String",
            "Vec", "Option", "Result", "Some", "None", "Ok", "Err",
        ];

        for ty in types {
            if ty.starts_with(prefix) && !completions.contains(&ty.to_string()) {
                completions.push(ty.to_string());
            }
        }

        completions.sort();
        completions
    }

    fn repl_commands(&self) -> Vec<ReplCommand> {
        vec![
            ReplCommand::new("ast", "Show AST for expression", cmd_ast)
                .with_help("Usage: :ast <expr>\n\nParse and display the syn AST for an expression."),

            ReplCommand::new("tokens", "Show tokens for expression", cmd_tokens)
                .with_help("Usage: :tokens <expr>\n\nShow the token stream for an expression."),
        ]
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Rust-specific REPL Commands
// ═══════════════════════════════════════════════════════════════════════

fn cmd_ast(
    args: &str,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    let source = args.trim();
    if source.is_empty() {
        return Ok(ReplCommandResult::Text("Usage: :ast <expr>".to_string()));
    }

    match syn::parse_str::<syn::Expr>(source) {
        Ok(expr) => Ok(ReplCommandResult::Text(format!("{:#?}", expr))),
        Err(e) => Ok(ReplCommandResult::Text(format!("Parse error: {}", e))),
    }
}

fn cmd_tokens(
    args: &str,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    let source = args.trim();
    if source.is_empty() {
        return Ok(ReplCommandResult::Text("Usage: :tokens <expr>".to_string()));
    }

    match source.parse::<proc_macro2::TokenStream>() {
        Ok(tokens) => {
            let mut output = String::new();
            for token in tokens {
                output.push_str(&format!("{:?}\n", token));
            }
            Ok(ReplCommandResult::Text(output))
        }
        Err(e) => Ok(ReplCommandResult::Text(format!("Tokenize error: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Syntax Highlighting
// ═══════════════════════════════════════════════════════════════════════

/// ANSI color codes
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const KEYWORD: &str = "\x1b[94m";      // Blue
    pub const STRING: &str = "\x1b[32m";       // Green
    pub const NUMBER: &str = "\x1b[33m";       // Yellow
    pub const COMMENT: &str = "\x1b[90m";      // Gray
    pub const TYPE: &str = "\x1b[36m";         // Cyan
    pub const FUNCTION: &str = "\x1b[93m";     // Bright yellow
    pub const MACRO: &str = "\x1b[35m";        // Magenta
}

fn highlight_rust(source: &str) -> String {
    let keywords = [
        "let", "mut", "fn", "if", "else", "match", "loop", "while", "for",
        "in", "break", "continue", "return", "struct", "enum", "impl",
        "trait", "type", "const", "static", "pub", "use", "mod", "self",
        "true", "false", "as", "ref", "where", "async", "await", "move",
    ];

    let types = [
        "i8", "i16", "i32", "i64", "i128", "isize",
        "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64", "bool", "char", "str", "String",
        "Vec", "Option", "Result", "Box", "Rc", "Arc",
        "Some", "None", "Ok", "Err", "Self",
    ];

    let mut result = String::new();
    let mut chars = source.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Comments
            '/' if chars.peek() == Some(&'/') => {
                result.push_str(colors::COMMENT);
                result.push(c);
                while let Some(ch) = chars.next() {
                    result.push(ch);
                    if ch == '\n' {
                        break;
                    }
                }
                result.push_str(colors::RESET);
            }

            // Strings
            '"' => {
                result.push_str(colors::STRING);
                result.push(c);
                let mut escape = false;
                while let Some(ch) = chars.next() {
                    result.push(ch);
                    if escape {
                        escape = false;
                    } else if ch == '\\' {
                        escape = true;
                    } else if ch == '"' {
                        break;
                    }
                }
                result.push_str(colors::RESET);
            }

            // Character literals
            '\'' => {
                result.push_str(colors::STRING);
                result.push(c);
                if let Some(ch) = chars.next() {
                    result.push(ch);
                    if ch == '\\' {
                        if let Some(esc) = chars.next() {
                            result.push(esc);
                        }
                    }
                }
                if let Some('\'') = chars.peek() {
                    result.push(chars.next().unwrap());
                }
                result.push_str(colors::RESET);
            }

            // Numbers
            '0'..='9' => {
                result.push_str(colors::NUMBER);
                result.push(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                        result.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str(colors::RESET);
            }

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::new();
                ident.push(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Check if it's a macro (followed by !)
                if chars.peek() == Some(&'!') {
                    result.push_str(colors::MACRO);
                    result.push_str(&ident);
                    result.push(chars.next().unwrap()); // !
                    result.push_str(colors::RESET);
                } else if keywords.contains(&ident.as_str()) {
                    result.push_str(colors::KEYWORD);
                    result.push_str(&ident);
                    result.push_str(colors::RESET);
                } else if types.contains(&ident.as_str()) {
                    result.push_str(colors::TYPE);
                    result.push_str(&ident);
                    result.push_str(colors::RESET);
                } else {
                    result.push_str(&ident);
                }
            }

            // Everything else
            _ => result.push(c),
        }
    }

    result
}

// ═══════════════════════════════════════════════════════════════════════
// Helper: Parse File
// ═══════════════════════════════════════════════════════════════════════

impl RustFrontend {
    /// Parse a file from path.
    pub fn parse_file(&self, path: &std::path::Path) -> Result<Vec<syn::Item>, ParseError> {
        let source = std::fs::read_to_string(path).map_err(|e| ParseError::Syntax {
            message: format!("failed to read file: {}", e),
            line: 0,
            column: 0,
            source_snippet: None,
        })?;

        self.parse(&source)
    }

    /// Create an evaluator session with this frontend.
    pub fn session(&self) -> RustSession {
        RustSession::new()
    }
}

/// A REPL session using the Rust frontend.
pub struct RustSession {
    pub env: Environment,
    pub ctx: EvalContext,
    pub frontend: RustFrontend,
}

impl RustSession {
    pub fn new() -> Self {
        Self {
            env: Environment::with_prelude(),
            ctx: EvalContext::default(),
            frontend: RustFrontend::new(),
        }
    }

    /// Evaluate a Rust expression.
    pub fn eval_expr(&mut self, source: &str) -> Result<Value, EvalError> {
        let expr = self.frontend.parse_expr(source)?;
        use crate::Evaluate;
        expr.eval(&mut self.env, &self.ctx)
    }

    /// Evaluate Rust items (fn, struct, etc.).
    pub fn eval_items(&mut self, source: &str) -> Result<Value, EvalError> {
        let items = self.frontend.parse(source)?;
        crate::eval_items(&items, &mut self.env, &self.ctx)
    }

    /// Evaluate source that may be items or an expression.
    pub fn eval(&mut self, source: &str) -> Result<Value, EvalError> {
        // Try as items first
        if let Ok(items) = self.frontend.parse(source) {
            if !items.is_empty() {
                return crate::eval_items(&items, &mut self.env, &self.ctx);
            }
        }

        // Try as expression
        self.eval_expr(source)
    }
}

impl Default for RustSession {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## Update Frontend Module

### Update src/frontend/mod.rs

```rust
pub mod error;
pub mod source_map;
pub mod repl_command;
pub mod rust;  // ← Add this

pub use error::{FrontendError, ParseError, MacroError};
pub use source_map::{SourceMap, SourceLocation};
pub use repl_command::{ReplCommand, ReplCommandHandler, ReplCommandResult, standard_commands};
pub use rust::{RustFrontend, RustSession};  // ← Add this

// ... rest of mod.rs unchanged ...
```

---

## Update lib.rs

### Add to exports

```rust
pub use frontend::{
    // ... existing exports ...
    RustFrontend,
    RustSession,
};
```

---

## Test Cases

### tests/rust_frontend_tests.rs

```rust
use treebeard_core::*;

// ═══════════════════════════════════════════════════════════════════════
// Basic Parsing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_empty() {
    let frontend = RustFrontend::new();
    let items = frontend.parse("").unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_parse_function() {
    let frontend = RustFrontend::new();
    let items = frontend.parse("fn foo() {}").unwrap();
    assert_eq!(items.len(), 1);
    assert!(matches!(items[0], syn::Item::Fn(_)));
}

#[test]
fn test_parse_multiple_items() {
    let frontend = RustFrontend::new();
    let items = frontend.parse(r#"
        fn foo() {}
        fn bar() {}
        struct Point { x: i64, y: i64 }
    "#).unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn test_parse_expression() {
    let frontend = RustFrontend::new();
    let expr = frontend.parse_expr("1 + 2 * 3").unwrap();
    assert!(matches!(expr, syn::Expr::Binary(_)));
}

#[test]
fn test_parse_invalid() {
    let frontend = RustFrontend::new();
    let result = frontend.parse("fn foo( {}");
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════
// Frontend Metadata
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_name() {
    let frontend = RustFrontend::new();
    assert_eq!(frontend.name(), "Rust");
}

#[test]
fn test_extension() {
    let frontend = RustFrontend::new();
    assert_eq!(frontend.file_extension(), "rs");
}

#[test]
fn test_prompt() {
    let frontend = RustFrontend::new();
    assert_eq!(frontend.prompt(), "rust> ");
}

// ═══════════════════════════════════════════════════════════════════════
// Value Formatting
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_format_unit() {
    let frontend = RustFrontend::new();
    assert_eq!(frontend.format_value(&Value::Unit), "()");
}

#[test]
fn test_format_integers() {
    let frontend = RustFrontend::new();
    assert_eq!(frontend.format_value(&Value::I64(42)), "42");
    assert_eq!(frontend.format_value(&Value::I32(42)), "42i32");
    assert_eq!(frontend.format_value(&Value::U8(255)), "255u8");
}

#[test]
fn test_format_float() {
    let frontend = RustFrontend::new();
    assert_eq!(frontend.format_value(&Value::F64(3.14)), "3.14");
    assert_eq!(frontend.format_value(&Value::F64(1.0)), "1.0");
}

#[test]
fn test_format_string() {
    let frontend = RustFrontend::new();
    assert_eq!(frontend.format_value(&Value::string("hello")), "\"hello\"");
}

#[test]
fn test_format_tuple() {
    let frontend = RustFrontend::new();
    let tuple = Value::Tuple(std::sync::Arc::new(vec![
        Value::I64(1),
        Value::I64(2),
    ]));
    assert_eq!(frontend.format_value(&tuple), "(1, 2)");
}

#[test]
fn test_format_option() {
    let frontend = RustFrontend::new();
    assert_eq!(
        frontend.format_value(&Value::Option(Some(Box::new(Value::I64(42))))),
        "Some(42)"
    );
    assert_eq!(frontend.format_value(&Value::Option(None)), "None");
}

// ═══════════════════════════════════════════════════════════════════════
// Complete Input Detection
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_complete_simple() {
    let frontend = RustFrontend::new();
    assert!(frontend.is_complete_input("1 + 2"));
    assert!(frontend.is_complete_input("fn foo() {}"));
}

#[test]
fn test_incomplete_brace() {
    let frontend = RustFrontend::new();
    assert!(!frontend.is_complete_input("fn foo() {"));
    assert!(!frontend.is_complete_input("{ let x = 1;"));
}

#[test]
fn test_incomplete_paren() {
    let frontend = RustFrontend::new();
    assert!(!frontend.is_complete_input("foo(1, 2"));
    assert!(!frontend.is_complete_input("(1 + 2"));
}

#[test]
fn test_incomplete_string() {
    let frontend = RustFrontend::new();
    assert!(!frontend.is_complete_input("\"hello"));
}

#[test]
fn test_empty_complete() {
    let frontend = RustFrontend::new();
    assert!(frontend.is_complete_input(""));
    assert!(frontend.is_complete_input("   "));
}

// ═══════════════════════════════════════════════════════════════════════
// Completions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_keyword_completions() {
    let frontend = RustFrontend::new();
    let env = Environment::new();
    
    let completions = frontend.completions("le", &env);
    assert!(completions.contains(&"let".to_string()));
}

#[test]
fn test_type_completions() {
    let frontend = RustFrontend::new();
    let env = Environment::new();
    
    let completions = frontend.completions("St", &env);
    assert!(completions.contains(&"String".to_string()));
}

#[test]
fn test_env_completions() {
    let frontend = RustFrontend::new();
    let mut env = Environment::new();
    env.define("my_variable", Value::I64(42));
    
    let completions = frontend.completions("my_", &env);
    assert!(completions.contains(&"my_variable".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════
// Session
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_session_eval_expr() {
    let mut session = RustSession::new();
    let result = session.eval_expr("1 + 2").unwrap();
    assert_eq!(result, Value::I64(3));
}

#[test]
fn test_session_eval_items() {
    let mut session = RustSession::new();
    session.eval_items("fn add(a: i64, b: i64) -> i64 { a + b }").unwrap();
    let result = session.eval_expr("add(3, 4)").unwrap();
    assert_eq!(result, Value::I64(7));
}

#[test]
fn test_session_stateful() {
    let mut session = RustSession::new();
    
    // Define a function
    session.eval_items("fn double(x: i64) -> i64 { x * 2 }").unwrap();
    
    // Define a constant
    session.eval_items("const N: i64 = 21;").unwrap();
    
    // Use them together
    let result = session.eval_expr("double(N)").unwrap();
    assert_eq!(result, Value::I64(42));
}

// ═══════════════════════════════════════════════════════════════════════
// REPL Commands
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_rust_commands() {
    let frontend = RustFrontend::new();
    let commands = frontend.repl_commands();
    
    let names: Vec<_> = commands.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"ast"));
    assert!(names.contains(&"tokens"));
}

// ═══════════════════════════════════════════════════════════════════════
// Highlighting
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_highlight_returns_string() {
    let frontend = RustFrontend::new();
    let highlighted = frontend.highlight("let x = 42;");
    // Should contain the original text (possibly with ANSI codes)
    assert!(highlighted.contains("let"));
    assert!(highlighted.contains("42"));
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: Full Evaluation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_full_program() {
    let mut session = RustSession::new();
    
    session.eval_items(r#"
        fn factorial(n: i64) -> i64 {
            if n <= 1 { 1 } else { n * factorial(n - 1) }
        }
    "#).unwrap();
    
    let result = session.eval_expr("factorial(5)").unwrap();
    assert_eq!(result, Value::I64(120));
}

#[test]
fn test_struct_usage() {
    let mut session = RustSession::new();
    
    session.eval_items("struct Point { x: i64, y: i64 }").unwrap();
    
    let result = session.eval_expr("Point { x: 3, y: 4 }").unwrap();
    assert!(matches!(result, Value::Struct(_)));
    
    let result = session.eval_expr("{ let p = Point { x: 3, y: 4 }; p.x + p.y }").unwrap();
    assert_eq!(result, Value::I64(7));
}
```

---

## Completion Checklist

- [ ] Create `src/frontend/rust.rs` with `RustFrontend`
- [ ] Implement `LanguageFrontend` required methods (parse, name, file_extension)
- [ ] Implement `parse_expr` using `syn::parse_str`
- [ ] Implement `format_value` with Rust-style output
- [ ] Implement `format_value_pretty` for structured output
- [ ] Implement `is_complete_input` with bracket counting
- [ ] Implement `highlight` with basic syntax coloring
- [ ] Implement `completions` with keywords, types, and env names
- [ ] Add `:ast` and `:tokens` REPL commands
- [ ] Create `RustSession` convenience wrapper
- [ ] Update `frontend/mod.rs` exports
- [ ] Update `lib.rs` exports
- [ ] All tests passing

---

## Design Notes

### Why So Simple?

The Rust frontend is intentionally minimal because `syn` does all the work. This validates that our trait design doesn't impose unnecessary burden on frontend implementors.

### Why RustSession?

`RustSession` bundles `Environment`, `EvalContext`, and `RustFrontend` together for convenience. It's the quickest way to get a working Rust evaluator.

### Why Custom format_value?

Different frontends want different output styles:
- Rust: `Some(42)`, `Point { x: 1, y: 2 }`
- Oxur (upcoming): `(some 42)`, `#Point{x 1 y 2}`

Letting each frontend control formatting enables language-appropriate output.

### Why Syntax Highlighting?

Modern REPLs have syntax highlighting. While not essential, it significantly improves the user experience. The highlighting is simple keyword-based coloring.

---

## Next Stage

**Stage 2.3: Oxur Frontend** — Integrate Oxur's existing AST Bridge as a `LanguageFrontend` implementation, enabling S-expression syntax for Treebeard.
