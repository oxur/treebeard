# Stage 2.3: Oxur Frontend

**Phase:** 2 - Frontend Trait  
**Stage:** 2.3  
**Prerequisites:** Stage 2.1 (Frontend Trait), Stage 2.2 (Rust Frontend)  
**Estimated effort:** 2-3 days

---

## Objective

Integrate Oxur's existing AST Bridge as a `LanguageFrontend` implementation, enabling S-expression syntax for Treebeard. This proves the frontend trait design works with a fundamentally different syntax.

---

## Overview

The Oxur frontend transforms S-expressions into `syn` AST:

```
S-expression Source → [Reader] → S-exp Tree → [AST Bridge] → syn AST → Treebeard
        ↓                                          ↓
   "(+ 1 2)"                              syn::ExprBinary
```

**Key insight:** The AST Bridge already exists (95% complete). This stage is primarily glue code connecting existing components to the `LanguageFrontend` trait.

---

## Oxur Syntax Quick Reference

| Rust | Oxur | Notes |
|------|------|-------|
| `1 + 2` | `(+ 1 2)` | Prefix notation |
| `fn add(a: i64, b: i64) -> i64 { a + b }` | `(defn add [a:i64 b:i64] -> i64 (+ a b))` | Function definition |
| `if x > 0 { 1 } else { 0 }` | `(if (> x 0) 1 0)` | Conditional |
| `let x = 42;` | `(let [x 42] ...)` | Let binding |
| `Point { x: 1, y: 2 }` | `#Point{x 1 y 2}` | Struct literal |
| `vec![1, 2, 3]` | `[1 2 3]` | Vector literal |
| `"hello"` | `"hello"` | String (same) |
| `x.len()` | `(.len x)` | Method call |
| `foo(1, 2)` | `(foo 1 2)` | Function call |

---

## File Structure

```
oxur-runtime/src/
├── lib.rs              # Re-exports
├── frontend/
│   ├── mod.rs          # ← New: OxurFrontend
│   ├── reader.rs       # Existing: S-expression lexer/parser
│   ├── bridge.rs       # Existing: S-exp → syn conversion
│   ├── printer.rs      # Existing: Value → S-exp conversion
│   └── source_map.rs   # ← New: Span mapping
└── ...
```

---

## OxurFrontend Implementation

### src/frontend/mod.rs

```rust
pub mod reader;
pub mod bridge;
pub mod printer;
pub mod source_map;

use treebeard_core::{
    LanguageFrontend, ParseError, MacroError, MacroEnvironment,
    SourceMap, SourceLocation, ReplCommand, ReplCommandResult,
    Value, Environment, EvalContext, EvalError,
};

use reader::OxurReader;
use bridge::AstBridge;
use printer::SexpPrinter;
use source_map::OxurSourceMap;

/// Oxur language frontend for Treebeard.
///
/// Oxur is a Lisp-like syntax that compiles to Rust semantics.
/// This frontend uses the existing AST Bridge (95% complete) to
/// convert S-expressions to `syn` AST.
pub struct OxurFrontend {
    /// S-expression reader (lexer + parser)
    reader: OxurReader,

    /// S-exp to syn AST bridge
    bridge: AstBridge,

    /// Value to S-exp printer
    printer: SexpPrinter,

    /// Source map for error reporting
    source_map: OxurSourceMap,
}

impl OxurFrontend {
    pub fn new() -> Self {
        Self {
            reader: OxurReader::new(),
            bridge: AstBridge::new(),
            printer: SexpPrinter::new(),
            source_map: OxurSourceMap::new(),
        }
    }

    /// Parse and convert S-expressions to syn items.
    fn parse_sexps(&self, source: &str) -> Result<(Vec<Sexp>, OxurSourceMap), ParseError> {
        let mut source_map = OxurSourceMap::new();
        let sexps = self.reader.read_with_positions(source, &mut source_map)?;
        Ok((sexps, source_map))
    }
}

impl Default for OxurFrontend {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageFrontend for OxurFrontend {
    // ═══════════════════════════════════════════════════════════════════
    // Required Methods
    // ═══════════════════════════════════════════════════════════════════

    fn parse(&self, source: &str) -> Result<Vec<syn::Item>, ParseError> {
        // 1. Parse S-expressions
        let sexps = self.reader.read(source)?;

        // 2. Convert to syn AST via existing bridge
        let items = self.bridge.sexps_to_items(&sexps)?;

        Ok(items)
    }

    fn name(&self) -> &str {
        "Oxur"
    }

    fn file_extension(&self) -> &str {
        "oxur"
    }

    // ═══════════════════════════════════════════════════════════════════
    // Overridden Methods
    // ═══════════════════════════════════════════════════════════════════

    fn parse_expr(&self, source: &str) -> Result<syn::Expr, ParseError> {
        // Parse a single S-expression as an expression
        let sexps = self.reader.read(source)?;

        if sexps.is_empty() {
            return Err(ParseError::UnexpectedEof {
                context: "expected expression".to_string(),
            });
        }

        if sexps.len() > 1 {
            return Err(ParseError::Syntax {
                message: "expected single expression, got multiple forms".to_string(),
                line: 1,
                column: 0,
                source_snippet: Some(source.to_string()),
            });
        }

        self.bridge.sexp_to_expr(&sexps[0])
    }

    fn expand_macros(
        &self,
        items: Vec<syn::Item>,
        macro_env: &mut MacroEnvironment,
    ) -> Result<Vec<syn::Item>, MacroError> {
        // Macro expansion will be implemented in Phase 3
        // For now, just return items unchanged
        // This is a placeholder that allows the frontend to work
        // without macros initially
        let _ = macro_env;
        Ok(items)
    }

    fn supports_macros(&self) -> bool {
        // Will be true after Phase 3
        false
    }

    fn format_value(&self, value: &Value) -> String {
        self.printer.value_to_sexp(value)
    }

    fn format_value_pretty(&self, value: &Value, indent: usize) -> String {
        self.printer.value_to_sexp_pretty(value, indent)
    }

    fn format_error(&self, error: &EvalError, source: &str) -> String {
        // Map syn spans back to S-expression positions
        if let Some(span) = error.span() {
            if let Some(loc) = self.source_map.syn_to_oxur(span) {
                return format_oxur_error(error, source, &loc);
            }
        }

        // Fallback: standard formatting
        format!("error: {}", error)
    }

    fn is_complete_input(&self, source: &str) -> bool {
        // Check for balanced parentheses/brackets
        let mut paren_depth = 0i32;
        let mut bracket_depth = 0i32;
        let mut in_string = false;
        let mut escape_next = false;

        for c in source.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '(' if !in_string => paren_depth += 1,
                ')' if !in_string => paren_depth -= 1,
                '[' if !in_string => bracket_depth += 1,
                ']' if !in_string => bracket_depth -= 1,
                _ => {}
            }
        }

        // Complete if all delimiters are balanced and not in string
        paren_depth == 0 && bracket_depth == 0 && !in_string
    }

    fn prompt(&self) -> &str {
        "oxur> "
    }

    fn continuation_prompt(&self) -> &str {
        "....| "
    }

    fn highlight(&self, source: &str) -> String {
        highlight_oxur(source)
    }

    fn completions(&self, prefix: &str, env: &Environment) -> Vec<String> {
        let mut completions: Vec<String> = env
            .all_names()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect();

        // Add Oxur special forms
        let special_forms = [
            "defn", "defmacro", "let", "if", "cond", "when", "unless",
            "do", "fn", "quote", "quasiquote", "unquote", "unquote-splicing",
            "loop", "recur", "match", "struct", "enum", "impl",
        ];

        for form in special_forms {
            if form.starts_with(prefix) && !completions.contains(&form.to_string()) {
                completions.push(form.to_string());
            }
        }

        // Add common functions
        let functions = [
            "+", "-", "*", "/", "%", "=", "!=", "<", ">", "<=", ">=",
            "and", "or", "not", "print", "println", "str", "type-of",
            "first", "rest", "cons", "list", "vec", "map", "filter",
            "reduce", "len", "empty?", "nil?", "some?",
        ];

        for func in functions {
            if func.starts_with(prefix) && !completions.contains(&func.to_string()) {
                completions.push(func.to_string());
            }
        }

        completions.sort();
        completions
    }

    fn repl_commands(&self) -> Vec<ReplCommand> {
        vec![
            ReplCommand::new("sexp", "Show S-expression parse", cmd_sexp)
                .with_help("Usage: :sexp <expr>\n\nParse and display the S-expression tree."),

            ReplCommand::new("syn", "Show syn AST", cmd_syn)
                .with_help("Usage: :syn <expr>\n\nShow the syn AST produced from the S-expression."),

            ReplCommand::new("rust", "Show equivalent Rust", cmd_rust)
                .with_help("Usage: :rust <expr>\n\nShow the equivalent Rust code."),

            // These will be enabled in Phase 3:
            // ReplCommand::new("macroexpand", "Expand macros", cmd_macroexpand),
            // ReplCommand::new("macroexpand-1", "Expand once", cmd_macroexpand_1),
        ]
    }

    fn new_macro_environment(&self) -> MacroEnvironment {
        let mut env = MacroEnvironment::new();

        // Register built-in macros (Phase 3 will add more)
        // For now, just create an empty environment

        env
    }
}

// ═══════════════════════════════════════════════════════════════════════
// S-expression Types (from existing oxur-reader)
// ═══════════════════════════════════════════════════════════════════════

/// S-expression representation.
#[derive(Debug, Clone, PartialEq)]
pub enum Sexp {
    /// Symbol: `foo`, `+`, `my-var`
    Symbol(String),

    /// Keyword: `:foo`, `:bar`
    Keyword(String),

    /// Integer: `42`, `-17`
    Int(i64),

    /// Float: `3.14`, `-2.5`
    Float(f64),

    /// String: `"hello"`
    String(String),

    /// Character: `\a`, `\newline`
    Char(char),

    /// Boolean: `true`, `false`
    Bool(bool),

    /// Nil: `nil`
    Nil,

    /// List: `(a b c)`
    List(Vec<Sexp>),

    /// Vector: `[a b c]`
    Vector(Vec<Sexp>),

    /// Map: `{:a 1 :b 2}`
    Map(Vec<(Sexp, Sexp)>),

    /// Tagged literal: `#Foo{...}`, `#Point[1 2]`
    Tagged(String, Box<Sexp>),

    /// Quote: `'x` → `(quote x)`
    Quote(Box<Sexp>),

    /// Quasiquote: `` `x `` → `(quasiquote x)`
    Quasiquote(Box<Sexp>),

    /// Unquote: `~x` → `(unquote x)`
    Unquote(Box<Sexp>),

    /// Unquote-splicing: `~@x` → `(unquote-splicing x)`
    UnquoteSplicing(Box<Sexp>),
}

impl Sexp {
    /// Check if this is a list starting with the given symbol.
    pub fn is_form(&self, name: &str) -> bool {
        match self {
            Sexp::List(items) => {
                matches!(items.first(), Some(Sexp::Symbol(s)) if s == name)
            }
            _ => false,
        }
    }

    /// Get as symbol string.
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Sexp::Symbol(s) => Some(s),
            _ => None,
        }
    }

    /// Get as list.
    pub fn as_list(&self) -> Option<&[Sexp]> {
        match self {
            Sexp::List(items) => Some(items),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Oxur-specific REPL Commands
// ═══════════════════════════════════════════════════════════════════════

fn cmd_sexp(
    args: &str,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    let source = args.trim();
    if source.is_empty() {
        return Ok(ReplCommandResult::Text("Usage: :sexp <expr>".to_string()));
    }

    let reader = OxurReader::new();
    match reader.read(source) {
        Ok(sexps) => {
            let output = sexps
                .iter()
                .map(|s| format!("{:#?}", s))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(ReplCommandResult::Text(output))
        }
        Err(e) => Ok(ReplCommandResult::Text(format!("Parse error: {:?}", e))),
    }
}

fn cmd_syn(
    args: &str,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    let source = args.trim();
    if source.is_empty() {
        return Ok(ReplCommandResult::Text("Usage: :syn <expr>".to_string()));
    }

    let frontend = OxurFrontend::new();
    match frontend.parse_expr(source) {
        Ok(expr) => Ok(ReplCommandResult::Text(format!("{:#?}", expr))),
        Err(e) => Ok(ReplCommandResult::Text(format!("Parse error: {:?}", e))),
    }
}

fn cmd_rust(
    args: &str,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<ReplCommandResult, EvalError> {
    let source = args.trim();
    if source.is_empty() {
        return Ok(ReplCommandResult::Text("Usage: :rust <expr>".to_string()));
    }

    let frontend = OxurFrontend::new();

    // Try as expression first
    if let Ok(expr) = frontend.parse_expr(source) {
        let rust_code = quote::quote!(#expr).to_string();
        return Ok(ReplCommandResult::Text(rust_code));
    }

    // Try as items
    match frontend.parse(source) {
        Ok(items) => {
            let rust_code = items
                .iter()
                .map(|item| quote::quote!(#item).to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            Ok(ReplCommandResult::Text(rust_code))
        }
        Err(e) => Ok(ReplCommandResult::Text(format!("Parse error: {:?}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Error Formatting
// ═══════════════════════════════════════════════════════════════════════

fn format_oxur_error(error: &EvalError, source: &str, loc: &SourceLocation) -> String {
    let mut output = String::new();

    output.push_str(&format!("error: {}\n", error));
    output.push_str(&format!("  --> <input>:{}:{}\n", loc.line, loc.column));

    // Show source context
    if let Some(line) = source.lines().nth(loc.line.saturating_sub(1)) {
        output.push_str("   |\n");
        output.push_str(&format!("{:3} | {}\n", loc.line, line));
        output.push_str(&format!("   | {}^\n", " ".repeat(loc.column)));
    }

    output
}

// ═══════════════════════════════════════════════════════════════════════
// Syntax Highlighting
// ═══════════════════════════════════════════════════════════════════════

mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const SPECIAL: &str = "\x1b[94m";      // Blue - special forms
    pub const STRING: &str = "\x1b[32m";       // Green
    pub const NUMBER: &str = "\x1b[33m";       // Yellow
    pub const COMMENT: &str = "\x1b[90m";      // Gray
    pub const KEYWORD: &str = "\x1b[36m";      // Cyan - :keywords
    pub const PAREN: &str = "\x1b[90m";        // Gray - parentheses
    pub const BUILTIN: &str = "\x1b[93m";      // Bright yellow
}

fn highlight_oxur(source: &str) -> String {
    let special_forms = [
        "defn", "defmacro", "let", "if", "cond", "when", "unless",
        "do", "fn", "quote", "quasiquote", "loop", "recur", "match",
        "struct", "enum", "impl", "def", "set!",
    ];

    let builtins = [
        "+", "-", "*", "/", "%", "=", "!=", "<", ">", "<=", ">=",
        "and", "or", "not", "print", "println", "str", "type-of",
    ];

    let mut result = String::new();
    let mut chars = source.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Comments
            ';' => {
                result.push_str(colors::COMMENT);
                result.push(c);
                while let Some(&ch) = chars.peek() {
                    if ch == '\n' {
                        break;
                    }
                    result.push(chars.next().unwrap());
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

            // Parentheses
            '(' | ')' | '[' | ']' | '{' | '}' => {
                result.push_str(colors::PAREN);
                result.push(c);
                result.push_str(colors::RESET);
            }

            // Keywords (:foo)
            ':' => {
                result.push_str(colors::KEYWORD);
                result.push(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                        result.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str(colors::RESET);
            }

            // Numbers
            '0'..='9' => {
                result.push_str(colors::NUMBER);
                result.push(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '.' || ch == '_' {
                        result.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str(colors::RESET);
            }

            // Negative numbers
            '-' if chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                result.push_str(colors::NUMBER);
                result.push(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '.' || ch == '_' {
                        result.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str(colors::RESET);
            }

            // Symbols
            'a'..='z' | 'A'..='Z' | '_' | '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '?' => {
                let mut symbol = String::new();
                symbol.push(c);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || "-_+*/<>=!?".contains(ch) {
                        symbol.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if special_forms.contains(&symbol.as_str()) {
                    result.push_str(colors::SPECIAL);
                    result.push_str(&symbol);
                    result.push_str(colors::RESET);
                } else if builtins.contains(&symbol.as_str()) {
                    result.push_str(colors::BUILTIN);
                    result.push_str(&symbol);
                    result.push_str(colors::RESET);
                } else {
                    result.push_str(&symbol);
                }
            }

            // Quote shortcuts
            '\'' | '`' | '~' => {
                result.push_str(colors::SPECIAL);
                result.push(c);
                result.push_str(colors::RESET);
            }

            // Everything else
            _ => result.push(c),
        }
    }

    result
}
```

---

## S-expression Reader (Interface)

### src/frontend/reader.rs (Interface to existing code)

```rust
use super::{Sexp, ParseError};
use super::source_map::OxurSourceMap;

/// S-expression reader (lexer + parser).
///
/// This wraps the existing Oxur reader implementation.
pub struct OxurReader {
    // Configuration options
    allow_trailing_comma: bool,
}

impl OxurReader {
    pub fn new() -> Self {
        Self {
            allow_trailing_comma: true,
        }
    }

    /// Read S-expressions from source.
    pub fn read(&self, source: &str) -> Result<Vec<Sexp>, ParseError> {
        // Delegate to existing implementation
        oxur_reader::read(source).map_err(|e| ParseError::Syntax {
            message: e.message,
            line: e.line,
            column: e.column,
            source_snippet: e.snippet,
        })
    }

    /// Read S-expressions and record positions in source map.
    pub fn read_with_positions(
        &self,
        source: &str,
        source_map: &mut OxurSourceMap,
    ) -> Result<Vec<Sexp>, ParseError> {
        // Delegate to existing implementation with position tracking
        oxur_reader::read_with_positions(source, |sexp, start, end| {
            source_map.record_sexp(sexp, start, end);
        }).map_err(|e| ParseError::Syntax {
            message: e.message,
            line: e.line,
            column: e.column,
            source_snippet: e.snippet,
        })
    }
}

impl Default for OxurReader {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## AST Bridge (Interface)

### src/frontend/bridge.rs (Interface to existing code)

```rust
use super::{Sexp, ParseError};

/// AST Bridge: S-expressions → syn AST.
///
/// This wraps the existing 95% complete Oxur AST bridge.
pub struct AstBridge {
    // Configuration
}

impl AstBridge {
    pub fn new() -> Self {
        Self {}
    }

    /// Convert S-expressions to syn items.
    pub fn sexps_to_items(&self, sexps: &[Sexp]) -> Result<Vec<syn::Item>, ParseError> {
        sexps
            .iter()
            .map(|sexp| self.sexp_to_item(sexp))
            .collect()
    }

    /// Convert a single S-expression to a syn item.
    pub fn sexp_to_item(&self, sexp: &Sexp) -> Result<syn::Item, ParseError> {
        // Delegate to existing bridge
        oxur_bridge::sexp_to_item(sexp).map_err(bridge_error_to_parse_error)
    }

    /// Convert a single S-expression to a syn expression.
    pub fn sexp_to_expr(&self, sexp: &Sexp) -> Result<syn::Expr, ParseError> {
        // Delegate to existing bridge
        oxur_bridge::sexp_to_expr(sexp).map_err(bridge_error_to_parse_error)
    }
}

impl Default for AstBridge {
    fn default() -> Self {
        Self::new()
    }
}

fn bridge_error_to_parse_error(e: oxur_bridge::BridgeError) -> ParseError {
    ParseError::Syntax {
        message: e.message,
        line: e.line.unwrap_or(0),
        column: e.column.unwrap_or(0),
        source_snippet: e.context,
    }
}
```

---

## S-expression Printer

### src/frontend/printer.rs

```rust
use treebeard_core::Value;
use super::Sexp;

/// Pretty printer for converting Values to S-expression strings.
pub struct SexpPrinter {
    // Configuration
    max_width: usize,
}

impl SexpPrinter {
    pub fn new() -> Self {
        Self { max_width: 80 }
    }

    /// Convert a Value to S-expression string.
    pub fn value_to_sexp(&self, value: &Value) -> String {
        match value {
            Value::Unit => "nil".to_string(),
            Value::Bool(true) => "true".to_string(),
            Value::Bool(false) => "false".to_string(),
            Value::Char(c) => format!("\\{}", char_name(*c)),
            Value::String(s) => format!("{:?}", s.as_str()),

            // Integers
            Value::I8(n) => format!("{}i8", n),
            Value::I16(n) => format!("{}i16", n),
            Value::I32(n) => format!("{}i32", n),
            Value::I64(n) => n.to_string(),
            Value::I128(n) => format!("{}i128", n),
            Value::Isize(n) => format!("{}isize", n),
            Value::U8(n) => format!("{}u8", n),
            Value::U16(n) => format!("{}u16", n),
            Value::U32(n) => format!("{}u32", n),
            Value::U64(n) => format!("{}u64", n),
            Value::U128(n) => format!("{}u128", n),
            Value::Usize(n) => format!("{}usize", n),

            // Floats
            Value::F32(n) => format!("{}f32", n),
            Value::F64(n) => {
                if n.fract() == 0.0 {
                    format!("{}.0", n)
                } else {
                    n.to_string()
                }
            }

            // Collections
            Value::Tuple(elements) => {
                if elements.is_empty() {
                    "nil".to_string()
                } else {
                    let inner: Vec<_> = elements.iter().map(|v| self.value_to_sexp(v)).collect();
                    format!("(tuple {})", inner.join(" "))
                }
            }
            Value::Array(elements) | Value::Vec(elements) => {
                let inner: Vec<_> = elements.iter().map(|v| self.value_to_sexp(v)).collect();
                format!("[{}]", inner.join(" "))
            }
            Value::HashMap(map) => {
                let inner: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("{} {}", self.value_to_sexp(k), self.value_to_sexp(v)))
                    .collect();
                format!("{{{}}}", inner.join(" "))
            }

            // Structs and Enums
            Value::Struct(s) => {
                let fields: Vec<_> = s
                    .fields
                    .iter()
                    .map(|(k, v)| format!(":{} {}", k, self.value_to_sexp(v)))
                    .collect();
                format!("#{}{{ {} }}", s.type_name, fields.join(" "))
            }
            Value::Enum(e) => match &e.data {
                treebeard_core::EnumData::Unit => {
                    format!("{}::{}", e.type_name, e.variant)
                }
                treebeard_core::EnumData::Tuple(elements) => {
                    let inner: Vec<_> = elements.iter().map(|v| self.value_to_sexp(v)).collect();
                    format!("({}::{} {})", e.type_name, e.variant, inner.join(" "))
                }
                treebeard_core::EnumData::Struct(fields) => {
                    let inner: Vec<_> = fields
                        .iter()
                        .map(|(k, v)| format!(":{} {}", k, self.value_to_sexp(v)))
                        .collect();
                    format!("#{}::{}{{ {} }}", e.type_name, e.variant, inner.join(" "))
                }
            },

            // Option and Result
            Value::Option(Some(v)) => format!("(some {})", self.value_to_sexp(v)),
            Value::Option(None) => "none".to_string(),
            Value::Result(Ok(v)) => format!("(ok {})", self.value_to_sexp(v)),
            Value::Result(Err(e)) => format!("(err {})", self.value_to_sexp(e)),

            // Functions
            Value::Function(f) => format!("#<fn {}>", f.name),
            Value::Closure(_) => "#<closure>".to_string(),
            Value::BuiltinFn(f) => format!("#<builtin {}>", f.name),
            Value::CompiledFn(f) => format!("#<compiled {}>", f.name),

            // Other
            Value::Bytes(b) => format!("#bytes[{}]", b.len()),
            Value::Ref(r) => format!("(ref {})", self.value_to_sexp(&r.read())),
            Value::RefMut(r) => format!("(ref-mut {})", self.value_to_sexp(&r.read())),
        }
    }

    /// Pretty print with indentation.
    pub fn value_to_sexp_pretty(&self, value: &Value, indent: usize) -> String {
        let simple = self.value_to_sexp(value);
        if simple.len() <= self.max_width - indent * 2 {
            return format!("{}{}", "  ".repeat(indent), simple);
        }

        // Multi-line formatting for large values
        let prefix = "  ".repeat(indent);
        match value {
            Value::Vec(elements) | Value::Array(elements) => {
                let mut output = format!("{}[\n", prefix);
                for elem in elements.iter() {
                    output.push_str(&self.value_to_sexp_pretty(elem, indent + 1));
                    output.push('\n');
                }
                output.push_str(&format!("{}]", prefix));
                output
            }
            Value::Struct(s) => {
                let mut output = format!("{}#{}{{ \n", prefix, s.type_name);
                for (k, v) in &s.fields {
                    output.push_str(&format!("{}  :{} ", prefix, k));
                    output.push_str(&self.value_to_sexp(v));
                    output.push('\n');
                }
                output.push_str(&format!("{}}}", prefix));
                output
            }
            _ => format!("{}{}", prefix, simple),
        }
    }
}

impl Default for SexpPrinter {
    fn default() -> Self {
        Self::new()
    }
}

fn char_name(c: char) -> String {
    match c {
        '\n' => "newline".to_string(),
        '\r' => "return".to_string(),
        '\t' => "tab".to_string(),
        ' ' => "space".to_string(),
        _ => c.to_string(),
    }
}
```

---

## Oxur Source Map

### src/frontend/source_map.rs

```rust
use treebeard_core::SourceLocation;
use proc_macro2::Span;
use std::collections::HashMap;

/// Maps between Oxur source positions and syn spans.
#[derive(Debug, Default)]
pub struct OxurSourceMap {
    /// S-expression ID → Oxur source position
    sexp_positions: HashMap<u64, OxurPosition>,

    /// Syn span → Oxur position  
    syn_to_oxur_map: HashMap<SpanKey, SourceLocation>,

    /// Counter for S-expression IDs
    next_id: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct OxurPosition {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

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

impl OxurSourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an S-expression's position.
    pub fn record_sexp(
        &mut self,
        _sexp: &super::Sexp,
        start: (usize, usize),
        end: (usize, usize),
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.sexp_positions.insert(
            id,
            OxurPosition {
                start_line: start.0,
                start_col: start.1,
                end_line: end.0,
                end_col: end.1,
            },
        );

        id
    }

    /// Record a mapping from syn span to Oxur position.
    pub fn record_syn_span(&mut self, span: Span, oxur_pos: OxurPosition) {
        self.syn_to_oxur_map.insert(
            span.into(),
            SourceLocation {
                line: oxur_pos.start_line,
                column: oxur_pos.start_col,
                end_line: oxur_pos.end_line,
                end_column: oxur_pos.end_col,
                label: None,
            },
        );
    }

    /// Look up Oxur position from syn span.
    pub fn syn_to_oxur(&self, span: Span) -> Option<SourceLocation> {
        self.syn_to_oxur_map.get(&span.into()).cloned()
    }
}
```

---

## OxurSession

### Add to src/frontend/mod.rs

```rust
/// A REPL session using the Oxur frontend.
pub struct OxurSession {
    pub env: Environment,
    pub ctx: EvalContext,
    pub macro_env: MacroEnvironment,
    pub frontend: OxurFrontend,
}

impl OxurSession {
    pub fn new() -> Self {
        let frontend = OxurFrontend::new();
        Self {
            env: Environment::with_prelude(),
            ctx: EvalContext::default(),
            macro_env: frontend.new_macro_environment(),
            frontend,
        }
    }

    /// Evaluate an Oxur expression.
    pub fn eval_expr(&mut self, source: &str) -> Result<Value, EvalError> {
        let expr = self.frontend.parse_expr(source)?;
        use treebeard_core::Evaluate;
        expr.eval(&mut self.env, &self.ctx)
    }

    /// Evaluate Oxur items (defn, struct, etc.).
    pub fn eval_items(&mut self, source: &str) -> Result<Value, EvalError> {
        let items = self.frontend.parse(source)?;
        let items = self.frontend.expand_macros(items, &mut self.macro_env)?;
        treebeard_core::eval_items(&items, &mut self.env, &self.ctx)
    }

    /// Evaluate source (items or expression).
    pub fn eval(&mut self, source: &str) -> Result<Value, EvalError> {
        // Try as items first
        if let Ok(items) = self.frontend.parse(source) {
            if !items.is_empty() {
                let items = self.frontend.expand_macros(items, &mut self.macro_env)?;
                return treebeard_core::eval_items(&items, &mut self.env, &self.ctx);
            }
        }

        // Try as expression
        self.eval_expr(source)
    }
}

impl Default for OxurSession {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## Test Cases

### tests/oxur_frontend_tests.rs

```rust
use oxur_runtime::frontend::*;
use treebeard_core::*;

// ═══════════════════════════════════════════════════════════════════════
// Basic Parsing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_number() {
    let frontend = OxurFrontend::new();
    let expr = frontend.parse_expr("42").unwrap();
    assert!(matches!(expr, syn::Expr::Lit(_)));
}

#[test]
fn test_parse_addition() {
    let frontend = OxurFrontend::new();
    let expr = frontend.parse_expr("(+ 1 2)").unwrap();
    assert!(matches!(expr, syn::Expr::Binary(_)));
}

#[test]
fn test_parse_nested() {
    let frontend = OxurFrontend::new();
    let expr = frontend.parse_expr("(+ (* 2 3) 4)").unwrap();
    assert!(matches!(expr, syn::Expr::Binary(_)));
}

#[test]
fn test_parse_function() {
    let frontend = OxurFrontend::new();
    let items = frontend.parse("(defn add [a:i64 b:i64] -> i64 (+ a b))").unwrap();
    assert_eq!(items.len(), 1);
    assert!(matches!(items[0], syn::Item::Fn(_)));
}

#[test]
fn test_parse_multiple() {
    let frontend = OxurFrontend::new();
    let items = frontend.parse(r#"
        (defn foo [] 1)
        (defn bar [] 2)
    "#).unwrap();
    assert_eq!(items.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════
// Frontend Metadata
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_name() {
    let frontend = OxurFrontend::new();
    assert_eq!(frontend.name(), "Oxur");
}

#[test]
fn test_extension() {
    let frontend = OxurFrontend::new();
    assert_eq!(frontend.file_extension(), "oxur");
}

#[test]
fn test_prompt() {
    let frontend = OxurFrontend::new();
    assert_eq!(frontend.prompt(), "oxur> ");
}

// ═══════════════════════════════════════════════════════════════════════
// Value Formatting
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_format_nil() {
    let frontend = OxurFrontend::new();
    assert_eq!(frontend.format_value(&Value::Unit), "nil");
}

#[test]
fn test_format_number() {
    let frontend = OxurFrontend::new();
    assert_eq!(frontend.format_value(&Value::I64(42)), "42");
}

#[test]
fn test_format_vector() {
    let frontend = OxurFrontend::new();
    let vec = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
    assert_eq!(frontend.format_value(&vec), "[1 2 3]");
}

#[test]
fn test_format_option() {
    let frontend = OxurFrontend::new();
    assert_eq!(
        frontend.format_value(&Value::Option(Some(Box::new(Value::I64(42))))),
        "(some 42)"
    );
    assert_eq!(frontend.format_value(&Value::Option(None)), "none");
}

// ═══════════════════════════════════════════════════════════════════════
// Complete Input Detection
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_complete_simple() {
    let frontend = OxurFrontend::new();
    assert!(frontend.is_complete_input("42"));
    assert!(frontend.is_complete_input("(+ 1 2)"));
}

#[test]
fn test_incomplete_paren() {
    let frontend = OxurFrontend::new();
    assert!(!frontend.is_complete_input("(+ 1 2"));
    assert!(!frontend.is_complete_input("(defn foo ["));
}

#[test]
fn test_incomplete_string() {
    let frontend = OxurFrontend::new();
    assert!(!frontend.is_complete_input("\"hello"));
}

// ═══════════════════════════════════════════════════════════════════════
// Session
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_session_eval_expr() {
    let mut session = OxurSession::new();
    let result = session.eval_expr("(+ 1 2)").unwrap();
    assert_eq!(result, Value::I64(3));
}

#[test]
fn test_session_eval_function() {
    let mut session = OxurSession::new();
    session.eval_items("(defn add [a:i64 b:i64] -> i64 (+ a b))").unwrap();
    let result = session.eval_expr("(add 3 4)").unwrap();
    assert_eq!(result, Value::I64(7));
}

#[test]
fn test_session_stateful() {
    let mut session = OxurSession::new();
    
    session.eval_items("(defn double [x:i64] -> i64 (* x 2))").unwrap();
    session.eval_items("(def N 21)").unwrap();
    
    let result = session.eval_expr("(double N)").unwrap();
    assert_eq!(result, Value::I64(42));
}

// ═══════════════════════════════════════════════════════════════════════
// Cross-Frontend Equivalence
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_rust_oxur_equivalence() {
    let mut rust_session = treebeard_core::RustSession::new();
    let mut oxur_session = OxurSession::new();
    
    // Define equivalent functions
    rust_session.eval_items("fn add(a: i64, b: i64) -> i64 { a + b }").unwrap();
    oxur_session.eval_items("(defn add [a:i64 b:i64] -> i64 (+ a b))").unwrap();
    
    // Evaluate equivalent expressions
    let rust_result = rust_session.eval_expr("add(3, 4)").unwrap();
    let oxur_result = oxur_session.eval_expr("(add 3 4)").unwrap();
    
    assert_eq!(rust_result, oxur_result);
}

#[test]
fn test_factorial_equivalence() {
    let mut rust_session = treebeard_core::RustSession::new();
    let mut oxur_session = OxurSession::new();
    
    rust_session.eval_items(r#"
        fn factorial(n: i64) -> i64 {
            if n <= 1 { 1 } else { n * factorial(n - 1) }
        }
    "#).unwrap();
    
    oxur_session.eval_items(r#"
        (defn factorial [n:i64] -> i64
          (if (<= n 1) 1 (* n (factorial (- n 1)))))
    "#).unwrap();
    
    let rust_result = rust_session.eval_expr("factorial(5)").unwrap();
    let oxur_result = oxur_session.eval_expr("(factorial 5)").unwrap();
    
    assert_eq!(rust_result, Value::I64(120));
    assert_eq!(oxur_result, Value::I64(120));
}

// ═══════════════════════════════════════════════════════════════════════
// REPL Commands
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_oxur_commands() {
    let frontend = OxurFrontend::new();
    let commands = frontend.repl_commands();
    
    let names: Vec<_> = commands.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"sexp"));
    assert!(names.contains(&"syn"));
    assert!(names.contains(&"rust"));
}
```

---

## Completion Checklist

- [ ] Create `src/frontend/mod.rs` with `OxurFrontend`
- [ ] Implement `LanguageFrontend` required methods
- [ ] Connect to existing `OxurReader` (S-expression parser)
- [ ] Connect to existing `AstBridge` (S-exp → syn)
- [ ] Implement `SexpPrinter` for value formatting
- [ ] Implement `OxurSourceMap` for span mapping
- [ ] Implement `is_complete_input` for multi-line REPL
- [ ] Implement `highlight` for S-expression syntax
- [ ] Implement `completions` with special forms
- [ ] Add `:sexp`, `:syn`, `:rust` REPL commands
- [ ] Create `OxurSession` convenience wrapper
- [ ] Test cross-frontend equivalence
- [ ] All tests passing

---

## Design Notes

### Why So Much Existing Code?

The AST Bridge is 95% complete. This stage is primarily:
1. Glue code connecting existing pieces
2. The `LanguageFrontend` trait implementation
3. New convenience wrappers (Session, Printer)

### Why Placeholder Macros?

Macro expansion is Phase 3. We implement `expand_macros` as a pass-through so the frontend works immediately. Phase 3 will add real expansion.

### Why Test Cross-Frontend Equivalence?

The whole point of Treebeard is that different syntaxes produce identical results. Testing `factorial` in both Rust and Oxur proves the architecture works.

### Success Criteria Met!

```rust
// Same program, two syntaxes, same result:
let rust_result = rust_frontend.eval("fn add(a: i32, b: i32) -> i32 { a + b }")?;
let oxur_result = oxur_frontend.eval("(defn add [a:i32 b:i32] -> i32 (+ a b))")?;
assert_eq!(rust_result, oxur_result);
```

---

## Phase 2 Complete! 🎉

With this stage, Treebeard has:
- ✅ A clean `LanguageFrontend` trait
- ✅ A working Rust frontend
- ✅ A working Oxur frontend (using existing AST Bridge)
- ✅ Cross-syntax equivalence proven

---

## Next Phase

**Phase 3: Macro System** — Implement Lisp-style macros for Oxur:
- Stage 3.1: Macro Environment
- Stage 3.2: Quasiquote
- Stage 3.3: Defmacro
- Stage 3.4: Expansion Pass
- Stage 3.5: Hygiene
