# Stage 1.3: Basic Expressions

**Phase:** 1 - Core Evaluator
**Stage:** 1.3
**Prerequisites:** Stage 1.1 (Value), Stage 1.2 (Environment)
**Estimated effort:** 3-4 days

---

## Objective

Implement evaluation for basic expressions: literals, paths (variable references), binary operations, and unary operations. This stage establishes the `Evaluate` trait and the core expression evaluation infrastructure.

---

## Overview

This stage covers four `syn` expression types:

| Type | Example | Evaluates To |
|------|---------|--------------|
| `syn::ExprLit` | `42`, `"hello"`, `true` | Direct `Value` conversion |
| `syn::ExprPath` | `x`, `foo` | Environment lookup |
| `syn::ExprUnary` | `-x`, `!flag` | Unary operation on evaluated operand |
| `syn::ExprBinary` | `a + b`, `x == y` | Binary operation on evaluated operands |

We also introduce:

- The `Evaluate` trait for AST node evaluation
- `EvalContext` for evaluation configuration
- `EvalError` for evaluation failures with spans
- The dispatcher for `syn::Expr` (routes to specific implementations)

---

## File Structure

```
treebeard/src/
├── lib.rs              # Add: pub mod eval; pub mod context;
├── value.rs            # From Stage 1.1
├── environment.rs      # From Stage 1.2
├── error.rs            # Extend with EvalError
├── context.rs          # ← New: EvalContext
└── eval/
    ├── mod.rs          # Evaluate trait + dispatcher
    ├── literal.rs      # syn::ExprLit
    ├── path.rs         # syn::ExprPath
    ├── unary.rs        # syn::ExprUnary
    └── binary.rs       # syn::ExprBinary
```

---

## EvalContext

### src/context.rs

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Configuration and state for evaluation.
///
/// This is passed through all evaluation calls and controls
/// behavior like recursion limits and interruption.
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Maximum call depth (stack overflow protection)
    pub max_call_depth: usize,

    /// Interrupt flag - set to true to abort evaluation
    pub interrupt: Arc<AtomicBool>,

    /// Whether to trace evaluation (for debugging)
    pub trace: bool,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            max_call_depth: 1000,
            interrupt: Arc::new(AtomicBool::new(false)),
            trace: false,
        }
    }
}

impl EvalContext {
    /// Create a new context with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with a custom call depth limit.
    pub fn with_max_call_depth(max_depth: usize) -> Self {
        Self {
            max_call_depth: max_depth,
            ..Default::default()
        }
    }

    /// Check if evaluation has been interrupted.
    pub fn is_interrupted(&self) -> bool {
        self.interrupt.load(Ordering::Relaxed)
    }

    /// Request interruption of evaluation.
    pub fn interrupt(&self) {
        self.interrupt.store(true, Ordering::Relaxed);
    }

    /// Reset the interrupt flag.
    pub fn reset_interrupt(&self) {
        self.interrupt.store(false, Ordering::Relaxed);
    }
}
```

---

## EvalError

### Extend src/error.rs

```rust
use proc_macro2::Span;
use thiserror::Error;

// ... keep existing EnvironmentError ...

/// Errors that can occur during evaluation.
#[derive(Error, Debug, Clone)]
pub enum EvalError {
    /// Undefined variable reference.
    #[error("undefined variable `{name}`")]
    UndefinedVariable {
        name: String,
        #[source]
        span: Option<Span>,
    },

    /// Type mismatch in operation.
    #[error("type error: {message}")]
    TypeError {
        message: String,
        span: Option<Span>,
    },

    /// Division by zero.
    #[error("division by zero")]
    DivisionByZero { span: Option<Span> },

    /// Integer overflow.
    #[error("integer overflow")]
    IntegerOverflow { span: Option<Span> },

    /// Invalid operand for unary operator.
    #[error("cannot apply `{op}` to {operand_type}")]
    InvalidUnaryOperand {
        op: String,
        operand_type: String,
        span: Option<Span>,
    },

    /// Invalid operands for binary operator.
    #[error("cannot apply `{op}` to {left_type} and {right_type}")]
    InvalidBinaryOperands {
        op: String,
        left_type: String,
        right_type: String,
        span: Option<Span>,
    },

    /// Unsupported expression type.
    #[error("unsupported expression: {kind}")]
    UnsupportedExpr {
        kind: String,
        span: Option<Span>,
    },

    /// Unsupported literal type.
    #[error("unsupported literal: {kind}")]
    UnsupportedLiteral {
        kind: String,
        span: Option<Span>,
    },

    /// Evaluation was interrupted.
    #[error("evaluation interrupted")]
    Interrupted,

    /// Stack overflow (too much recursion).
    #[error("stack overflow: maximum call depth ({max}) exceeded")]
    StackOverflow { max: usize },

    /// Environment error wrapper.
    #[error(transparent)]
    Environment(#[from] EnvironmentError),
}

impl EvalError {
    /// Get the source span for this error, if available.
    pub fn span(&self) -> Option<Span> {
        match self {
            EvalError::UndefinedVariable { span, .. } => *span,
            EvalError::TypeError { span, .. } => *span,
            EvalError::DivisionByZero { span } => *span,
            EvalError::IntegerOverflow { span } => *span,
            EvalError::InvalidUnaryOperand { span, .. } => *span,
            EvalError::InvalidBinaryOperands { span, .. } => *span,
            EvalError::UnsupportedExpr { span, .. } => *span,
            EvalError::UnsupportedLiteral { span, .. } => *span,
            EvalError::Interrupted => None,
            EvalError::StackOverflow { .. } => None,
            EvalError::Environment(_) => None,
        }
    }
}

/// Helper to get a type name for error messages.
pub fn type_name(value: &crate::Value) -> &'static str {
    match value {
        crate::Value::Unit => "()",
        crate::Value::Bool(_) => "bool",
        crate::Value::Char(_) => "char",
        crate::Value::I8(_) => "i8",
        crate::Value::I16(_) => "i16",
        crate::Value::I32(_) => "i32",
        crate::Value::I64(_) => "i64",
        crate::Value::I128(_) => "i128",
        crate::Value::Isize(_) => "isize",
        crate::Value::U8(_) => "u8",
        crate::Value::U16(_) => "u16",
        crate::Value::U32(_) => "u32",
        crate::Value::U64(_) => "u64",
        crate::Value::U128(_) => "u128",
        crate::Value::Usize(_) => "usize",
        crate::Value::F32(_) => "f32",
        crate::Value::F64(_) => "f64",
        crate::Value::String(_) => "String",
        crate::Value::Bytes(_) => "Vec<u8>",
        crate::Value::Vec(_) => "Vec",
        crate::Value::Tuple(_) => "tuple",
        crate::Value::Array(_) => "array",
        crate::Value::Struct(_) => "struct",
        crate::Value::Enum(_) => "enum",
        crate::Value::HashMap(_) => "HashMap",
        crate::Value::Option(_) => "Option",
        crate::Value::Result(_) => "Result",
        crate::Value::Function(_) => "fn",
        crate::Value::Closure(_) => "closure",
        crate::Value::BuiltinFn(_) => "builtin_fn",
        crate::Value::CompiledFn(_) => "compiled_fn",
        crate::Value::Ref(_) => "&T",
        crate::Value::RefMut(_) => "&mut T",
    }
}
```

---

## Evaluate Trait and Dispatcher

### src/eval/mod.rs

```rust
pub mod literal;
pub mod path;
pub mod unary;
pub mod binary;

use crate::{Value, Environment, EvalContext, EvalError};

/// Trait for evaluating AST nodes to values.
///
/// This is the core abstraction for the tree-walking interpreter.
/// Each `syn` expression type implements this trait.
pub trait Evaluate {
    /// Evaluate this AST node in the given environment.
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError>;
}

// ═══════════════════════════════════════════════════════════════════════
// Main Expression Dispatcher
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::Expr {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Check for interruption before each expression
        if ctx.is_interrupted() {
            return Err(EvalError::Interrupted);
        }

        match self {
            // Stage 1.3: Basic expressions
            syn::Expr::Lit(expr) => expr.eval(env, ctx),
            syn::Expr::Path(expr) => expr.eval(env, ctx),
            syn::Expr::Unary(expr) => expr.eval(env, ctx),
            syn::Expr::Binary(expr) => expr.eval(env, ctx),

            // Stage 1.4: Control flow (not yet implemented)
            syn::Expr::If(_) => Err(not_yet_implemented("if expression", self)),
            syn::Expr::Match(_) => Err(not_yet_implemented("match expression", self)),
            syn::Expr::Loop(_) => Err(not_yet_implemented("loop expression", self)),
            syn::Expr::While(_) => Err(not_yet_implemented("while expression", self)),
            syn::Expr::ForLoop(_) => Err(not_yet_implemented("for loop", self)),
            syn::Expr::Break(_) => Err(not_yet_implemented("break", self)),
            syn::Expr::Continue(_) => Err(not_yet_implemented("continue", self)),

            // Stage 1.5: Functions (not yet implemented)
            syn::Expr::Call(_) => Err(not_yet_implemented("function call", self)),
            syn::Expr::MethodCall(_) => Err(not_yet_implemented("method call", self)),
            syn::Expr::Closure(_) => Err(not_yet_implemented("closure", self)),
            syn::Expr::Return(_) => Err(not_yet_implemented("return", self)),

            // Stage 1.6: Blocks (not yet implemented)
            syn::Expr::Block(_) => Err(not_yet_implemented("block", self)),

            // Parenthesized expressions - just unwrap
            syn::Expr::Paren(expr) => expr.expr.eval(env, ctx),

            // Group expressions (for precedence) - just unwrap
            syn::Expr::Group(expr) => expr.expr.eval(env, ctx),

            // Everything else
            _ => Err(EvalError::UnsupportedExpr {
                kind: expr_kind_name(self).to_string(),
                span: Some(expr_span(self)),
            }),
        }
    }
}

/// Get a human-readable name for an expression kind.
fn expr_kind_name(expr: &syn::Expr) -> &'static str {
    match expr {
        syn::Expr::Array(_) => "array",
        syn::Expr::Assign(_) => "assignment",
        syn::Expr::Async(_) => "async block",
        syn::Expr::Await(_) => "await",
        syn::Expr::Binary(_) => "binary operation",
        syn::Expr::Block(_) => "block",
        syn::Expr::Break(_) => "break",
        syn::Expr::Call(_) => "function call",
        syn::Expr::Cast(_) => "cast",
        syn::Expr::Closure(_) => "closure",
        syn::Expr::Const(_) => "const block",
        syn::Expr::Continue(_) => "continue",
        syn::Expr::Field(_) => "field access",
        syn::Expr::ForLoop(_) => "for loop",
        syn::Expr::Group(_) => "group",
        syn::Expr::If(_) => "if",
        syn::Expr::Index(_) => "index",
        syn::Expr::Infer(_) => "infer",
        syn::Expr::Let(_) => "let guard",
        syn::Expr::Lit(_) => "literal",
        syn::Expr::Loop(_) => "loop",
        syn::Expr::Macro(_) => "macro invocation",
        syn::Expr::Match(_) => "match",
        syn::Expr::MethodCall(_) => "method call",
        syn::Expr::Paren(_) => "parenthesized",
        syn::Expr::Path(_) => "path",
        syn::Expr::Range(_) => "range",
        syn::Expr::Reference(_) => "reference",
        syn::Expr::Repeat(_) => "repeat",
        syn::Expr::Return(_) => "return",
        syn::Expr::Struct(_) => "struct literal",
        syn::Expr::Try(_) => "try",
        syn::Expr::TryBlock(_) => "try block",
        syn::Expr::Tuple(_) => "tuple",
        syn::Expr::Unary(_) => "unary operation",
        syn::Expr::Unsafe(_) => "unsafe block",
        syn::Expr::Verbatim(_) => "verbatim",
        syn::Expr::While(_) => "while",
        syn::Expr::Yield(_) => "yield",
        _ => "unknown",
    }
}

/// Get the span of an expression.
fn expr_span(expr: &syn::Expr) -> proc_macro2::Span {
    use quote::ToTokens;
    expr.to_token_stream()
        .into_iter()
        .next()
        .map(|t| t.span())
        .unwrap_or_else(proc_macro2::Span::call_site)
}

/// Create a "not yet implemented" error.
fn not_yet_implemented(what: &str, expr: &syn::Expr) -> EvalError {
    EvalError::UnsupportedExpr {
        kind: format!("{} (not yet implemented)", what),
        span: Some(expr_span(expr)),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Convenience Functions
// ═══════════════════════════════════════════════════════════════════════

/// Evaluate an expression (convenience wrapper).
pub fn eval_expr(
    expr: &syn::Expr,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    expr.eval(env, ctx)
}
```

---

## Literal Evaluation

### src/eval/literal.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;

impl Evaluate for syn::ExprLit {
    fn eval(&self, _env: &mut Environment, _ctx: &EvalContext) -> Result<Value, EvalError> {
        eval_lit(&self.lit)
    }
}

/// Evaluate a literal to a Value.
pub fn eval_lit(lit: &syn::Lit) -> Result<Value, EvalError> {
    match lit {
        syn::Lit::Str(s) => Ok(Value::string(s.value())),

        syn::Lit::ByteStr(bs) => Ok(Value::bytes(bs.value())),

        syn::Lit::CStr(_) => Err(EvalError::UnsupportedLiteral {
            kind: "C string literal".to_string(),
            span: Some(lit.span()),
        }),

        syn::Lit::Byte(b) => Ok(Value::U8(b.value())),

        syn::Lit::Char(c) => Ok(Value::Char(c.value())),

        syn::Lit::Int(i) => eval_int_literal(i),

        syn::Lit::Float(f) => eval_float_literal(f),

        syn::Lit::Bool(b) => Ok(Value::Bool(b.value())),

        syn::Lit::Verbatim(_) => Err(EvalError::UnsupportedLiteral {
            kind: "verbatim literal".to_string(),
            span: Some(lit.span()),
        }),

        _ => Err(EvalError::UnsupportedLiteral {
            kind: "unknown literal".to_string(),
            span: Some(lit.span()),
        }),
    }
}

/// Evaluate an integer literal, respecting suffixes.
fn eval_int_literal(lit: &syn::LitInt) -> Result<Value, EvalError> {
    let suffix = lit.suffix();
    let span = Some(lit.span());

    // Parse based on suffix
    match suffix {
        "i8" => lit
            .base10_parse::<i8>()
            .map(Value::I8)
            .map_err(|_| overflow_error(span)),
        "i16" => lit
            .base10_parse::<i16>()
            .map(Value::I16)
            .map_err(|_| overflow_error(span)),
        "i32" => lit
            .base10_parse::<i32>()
            .map(Value::I32)
            .map_err(|_| overflow_error(span)),
        "i64" => lit
            .base10_parse::<i64>()
            .map(Value::I64)
            .map_err(|_| overflow_error(span)),
        "i128" => lit
            .base10_parse::<i128>()
            .map(Value::I128)
            .map_err(|_| overflow_error(span)),
        "isize" => lit
            .base10_parse::<isize>()
            .map(Value::Isize)
            .map_err(|_| overflow_error(span)),
        "u8" => lit
            .base10_parse::<u8>()
            .map(Value::U8)
            .map_err(|_| overflow_error(span)),
        "u16" => lit
            .base10_parse::<u16>()
            .map(Value::U16)
            .map_err(|_| overflow_error(span)),
        "u32" => lit
            .base10_parse::<u32>()
            .map(Value::U32)
            .map_err(|_| overflow_error(span)),
        "u64" => lit
            .base10_parse::<u64>()
            .map(Value::U64)
            .map_err(|_| overflow_error(span)),
        "u128" => lit
            .base10_parse::<u128>()
            .map(Value::U128)
            .map_err(|_| overflow_error(span)),
        "usize" => lit
            .base10_parse::<usize>()
            .map(Value::Usize)
            .map_err(|_| overflow_error(span)),
        "" => {
            // No suffix - default to i64 (like Rust's type inference default for integers)
            lit.base10_parse::<i64>()
                .map(Value::I64)
                .map_err(|_| overflow_error(span))
        }
        other => Err(EvalError::UnsupportedLiteral {
            kind: format!("integer with suffix `{}`", other),
            span,
        }),
    }
}

/// Evaluate a float literal, respecting suffixes.
fn eval_float_literal(lit: &syn::LitFloat) -> Result<Value, EvalError> {
    let suffix = lit.suffix();
    let span = Some(lit.span());

    match suffix {
        "f32" => lit
            .base10_parse::<f32>()
            .map(Value::F32)
            .map_err(|e| EvalError::TypeError {
                message: format!("invalid f32 literal: {}", e),
                span,
            }),
        "f64" | "" => {
            // No suffix defaults to f64
            lit.base10_parse::<f64>()
                .map(Value::F64)
                .map_err(|e| EvalError::TypeError {
                    message: format!("invalid f64 literal: {}", e),
                    span,
                })
        }
        other => Err(EvalError::UnsupportedLiteral {
            kind: format!("float with suffix `{}`", other),
            span,
        }),
    }
}

fn overflow_error(span: Option<proc_macro2::Span>) -> EvalError {
    EvalError::IntegerOverflow { span }
}
```

---

## Path Evaluation (Variable Lookup)

### src/eval/path.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;

impl Evaluate for syn::ExprPath {
    fn eval(&self, env: &mut Environment, _ctx: &EvalContext) -> Result<Value, EvalError> {
        // For now, we only support simple paths (single identifier)
        // Complex paths like `std::collections::HashMap` are not supported yet

        if self.path.segments.len() != 1 {
            return Err(EvalError::UnsupportedExpr {
                kind: format!(
                    "qualified path `{}`",
                    path_to_string(&self.path)
                ),
                span: Some(self.path.segments.first().unwrap().ident.span()),
            });
        }

        let segment = self.path.segments.first().unwrap();
        let name = segment.ident.to_string();

        // Check for path arguments (like `foo::<T>`)
        if !matches!(segment.arguments, syn::PathArguments::None) {
            return Err(EvalError::UnsupportedExpr {
                kind: format!("path with type arguments `{}`", name),
                span: Some(segment.ident.span()),
            });
        }

        // Look up in environment
        env.get(&name)
            .cloned()
            .ok_or_else(|| EvalError::UndefinedVariable {
                name,
                span: Some(segment.ident.span()),
            })
    }
}

/// Convert a syn::Path to a string for error messages.
fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}
```

---

## Unary Operations

### src/eval/unary.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use crate::error::type_name;
use super::Evaluate;

impl Evaluate for syn::ExprUnary {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let operand = self.expr.eval(env, ctx)?;
        let span = Some(self.op.span());

        match &self.op {
            syn::UnOp::Neg(_) => eval_neg(operand, span),
            syn::UnOp::Not(_) => eval_not(operand, span),
            syn::UnOp::Deref(_) => eval_deref(operand, span),
        }
    }
}

/// Evaluate unary negation (`-x`).
fn eval_neg(operand: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    match operand {
        // Signed integers
        Value::I8(n) => n
            .checked_neg()
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I16(n) => n
            .checked_neg()
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I32(n) => n
            .checked_neg()
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I64(n) => n
            .checked_neg()
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I128(n) => n
            .checked_neg()
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::Isize(n) => n
            .checked_neg()
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),

        // Floats (no overflow for negation)
        Value::F32(n) => Ok(Value::F32(-n)),
        Value::F64(n) => Ok(Value::F64(-n)),

        // Unsigned integers can't be negated
        other => Err(EvalError::InvalidUnaryOperand {
            op: "-".to_string(),
            operand_type: type_name(&other).to_string(),
            span,
        }),
    }
}

/// Evaluate logical/bitwise NOT (`!x`).
fn eval_not(operand: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    match operand {
        // Logical NOT for booleans
        Value::Bool(b) => Ok(Value::Bool(!b)),

        // Bitwise NOT for integers
        Value::I8(n) => Ok(Value::I8(!n)),
        Value::I16(n) => Ok(Value::I16(!n)),
        Value::I32(n) => Ok(Value::I32(!n)),
        Value::I64(n) => Ok(Value::I64(!n)),
        Value::I128(n) => Ok(Value::I128(!n)),
        Value::Isize(n) => Ok(Value::Isize(!n)),
        Value::U8(n) => Ok(Value::U8(!n)),
        Value::U16(n) => Ok(Value::U16(!n)),
        Value::U32(n) => Ok(Value::U32(!n)),
        Value::U64(n) => Ok(Value::U64(!n)),
        Value::U128(n) => Ok(Value::U128(!n)),
        Value::Usize(n) => Ok(Value::Usize(!n)),

        other => Err(EvalError::InvalidUnaryOperand {
            op: "!".to_string(),
            operand_type: type_name(&other).to_string(),
            span,
        }),
    }
}

/// Evaluate dereference (`*x`).
fn eval_deref(operand: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    match operand {
        Value::Ref(r) => Ok((*r.value).clone()),
        Value::RefMut(r) => {
            let guard = r.value.read().map_err(|_| EvalError::TypeError {
                message: "failed to acquire read lock on RefMut".to_string(),
                span,
            })?;
            Ok(guard.clone())
        }
        other => Err(EvalError::InvalidUnaryOperand {
            op: "*".to_string(),
            operand_type: type_name(&other).to_string(),
            span,
        }),
    }
}
```

---

## Binary Operations

### src/eval/binary.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use crate::error::type_name;
use super::Evaluate;

impl Evaluate for syn::ExprBinary {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Short-circuit evaluation for && and ||
        match &self.op {
            syn::BinOp::And(_) => return eval_and(&self.left, &self.right, env, ctx),
            syn::BinOp::Or(_) => return eval_or(&self.left, &self.right, env, ctx),
            _ => {}
        }

        // Evaluate both operands
        let left = self.left.eval(env, ctx)?;
        let right = self.right.eval(env, ctx)?;
        let span = Some(self.op.span());

        match &self.op {
            // Arithmetic
            syn::BinOp::Add(_) => eval_add(left, right, span),
            syn::BinOp::Sub(_) => eval_sub(left, right, span),
            syn::BinOp::Mul(_) => eval_mul(left, right, span),
            syn::BinOp::Div(_) => eval_div(left, right, span),
            syn::BinOp::Rem(_) => eval_rem(left, right, span),

            // Comparison
            syn::BinOp::Eq(_) => Ok(Value::Bool(left == right)),
            syn::BinOp::Ne(_) => Ok(Value::Bool(left != right)),
            syn::BinOp::Lt(_) => eval_lt(left, right, span),
            syn::BinOp::Le(_) => eval_le(left, right, span),
            syn::BinOp::Gt(_) => eval_gt(left, right, span),
            syn::BinOp::Ge(_) => eval_ge(left, right, span),

            // Bitwise
            syn::BinOp::BitAnd(_) => eval_bitand(left, right, span),
            syn::BinOp::BitOr(_) => eval_bitor(left, right, span),
            syn::BinOp::BitXor(_) => eval_bitxor(left, right, span),
            syn::BinOp::Shl(_) => eval_shl(left, right, span),
            syn::BinOp::Shr(_) => eval_shr(left, right, span),

            // Logical (already handled above with short-circuit)
            syn::BinOp::And(_) | syn::BinOp::Or(_) => unreachable!(),

            // Assignment operators (not handled in this stage)
            syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_) => Err(EvalError::UnsupportedExpr {
                kind: "assignment operator (not yet implemented)".to_string(),
                span,
            }),

            _ => Err(EvalError::UnsupportedExpr {
                kind: "unknown binary operator".to_string(),
                span,
            }),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Short-Circuit Logical Operators
// ═══════════════════════════════════════════════════════════════════════

fn eval_and(
    left: &syn::Expr,
    right: &syn::Expr,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let left_val = left.eval(env, ctx)?;
    match left_val {
        Value::Bool(false) => Ok(Value::Bool(false)), // Short-circuit
        Value::Bool(true) => {
            let right_val = right.eval(env, ctx)?;
            match right_val {
                Value::Bool(b) => Ok(Value::Bool(b)),
                other => Err(EvalError::InvalidBinaryOperands {
                    op: "&&".to_string(),
                    left_type: "bool".to_string(),
                    right_type: type_name(&other).to_string(),
                    span: None,
                }),
            }
        }
        other => Err(EvalError::InvalidBinaryOperands {
            op: "&&".to_string(),
            left_type: type_name(&other).to_string(),
            right_type: "?".to_string(),
            span: None,
        }),
    }
}

fn eval_or(
    left: &syn::Expr,
    right: &syn::Expr,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let left_val = left.eval(env, ctx)?;
    match left_val {
        Value::Bool(true) => Ok(Value::Bool(true)), // Short-circuit
        Value::Bool(false) => {
            let right_val = right.eval(env, ctx)?;
            match right_val {
                Value::Bool(b) => Ok(Value::Bool(b)),
                other => Err(EvalError::InvalidBinaryOperands {
                    op: "||".to_string(),
                    left_type: "bool".to_string(),
                    right_type: type_name(&other).to_string(),
                    span: None,
                }),
            }
        }
        other => Err(EvalError::InvalidBinaryOperands {
            op: "||".to_string(),
            left_type: type_name(&other).to_string(),
            right_type: "?".to_string(),
            span: None,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Arithmetic Operations
// ═══════════════════════════════════════════════════════════════════════

macro_rules! impl_checked_arith {
    ($name:ident, $op:ident, $op_str:expr, $overflow_err:expr) => {
        fn $name(
            left: Value,
            right: Value,
            span: Option<proc_macro2::Span>,
        ) -> Result<Value, EvalError> {
            match (left, right) {
                // Same-type integer operations
                (Value::I8(a), Value::I8(b)) => a
                    .$op(b)
                    .map(Value::I8)
                    .ok_or($overflow_err(span)),
                (Value::I16(a), Value::I16(b)) => a
                    .$op(b)
                    .map(Value::I16)
                    .ok_or($overflow_err(span)),
                (Value::I32(a), Value::I32(b)) => a
                    .$op(b)
                    .map(Value::I32)
                    .ok_or($overflow_err(span)),
                (Value::I64(a), Value::I64(b)) => a
                    .$op(b)
                    .map(Value::I64)
                    .ok_or($overflow_err(span)),
                (Value::I128(a), Value::I128(b)) => a
                    .$op(b)
                    .map(Value::I128)
                    .ok_or($overflow_err(span)),
                (Value::Isize(a), Value::Isize(b)) => a
                    .$op(b)
                    .map(Value::Isize)
                    .ok_or($overflow_err(span)),
                (Value::U8(a), Value::U8(b)) => a
                    .$op(b)
                    .map(Value::U8)
                    .ok_or($overflow_err(span)),
                (Value::U16(a), Value::U16(b)) => a
                    .$op(b)
                    .map(Value::U16)
                    .ok_or($overflow_err(span)),
                (Value::U32(a), Value::U32(b)) => a
                    .$op(b)
                    .map(Value::U32)
                    .ok_or($overflow_err(span)),
                (Value::U64(a), Value::U64(b)) => a
                    .$op(b)
                    .map(Value::U64)
                    .ok_or($overflow_err(span)),
                (Value::U128(a), Value::U128(b)) => a
                    .$op(b)
                    .map(Value::U128)
                    .ok_or($overflow_err(span)),
                (Value::Usize(a), Value::Usize(b)) => a
                    .$op(b)
                    .map(Value::Usize)
                    .ok_or($overflow_err(span)),

                // Float operations (no overflow check needed for add/sub/mul)
                (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a.$op(b))),
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a.$op(b))),

                (left, right) => Err(EvalError::InvalidBinaryOperands {
                    op: $op_str.to_string(),
                    left_type: type_name(&left).to_string(),
                    right_type: type_name(&right).to_string(),
                    span,
                }),
            }
        }
    };
}

// For floats, we need a different approach since they don't have checked_* methods
// that return Option. We'll handle floats separately.

fn eval_add(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match (&left, &right) {
        // String concatenation
        (Value::String(a), Value::String(b)) => {
            Ok(Value::string(format!("{}{}", a.as_str(), b.as_str())))
        }

        // Numeric addition
        _ => eval_add_numeric(left, right, span),
    }
}

fn eval_add_numeric(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a.checked_add(b).map(Value::I8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a.checked_add(b).map(Value::I16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a.checked_add(b).map(Value::I32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a.checked_add(b).map(Value::I64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a.checked_add(b).map(Value::I128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a.checked_add(b).map(Value::Isize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a.checked_add(b).map(Value::U8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a.checked_add(b).map(Value::U16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a.checked_add(b).map(Value::U32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a.checked_add(b).map(Value::U64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a.checked_add(b).map(Value::U128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a.checked_add(b).map(Value::Usize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a + b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a + b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "+".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_sub(left: Value, right: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a.checked_sub(b).map(Value::I8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a.checked_sub(b).map(Value::I16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a.checked_sub(b).map(Value::I32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a.checked_sub(b).map(Value::I64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a.checked_sub(b).map(Value::I128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a.checked_sub(b).map(Value::Isize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a.checked_sub(b).map(Value::U8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a.checked_sub(b).map(Value::U16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a.checked_sub(b).map(Value::U32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a.checked_sub(b).map(Value::U64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a.checked_sub(b).map(Value::U128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a.checked_sub(b).map(Value::Usize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a - b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a - b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "-".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_mul(left: Value, right: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a.checked_mul(b).map(Value::I8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a.checked_mul(b).map(Value::I16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a.checked_mul(b).map(Value::I32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a.checked_mul(b).map(Value::I64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a.checked_mul(b).map(Value::I128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a.checked_mul(b).map(Value::Isize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a.checked_mul(b).map(Value::U8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a.checked_mul(b).map(Value::U16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a.checked_mul(b).map(Value::U32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a.checked_mul(b).map(Value::U64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a.checked_mul(b).map(Value::U128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a.checked_mul(b).map(Value::Usize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a * b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a * b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "*".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_div(left: Value, right: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    // Check for division by zero
    let is_zero = match &right {
        Value::I8(0) | Value::I16(0) | Value::I32(0) | Value::I64(0) | Value::I128(0) | Value::Isize(0) => true,
        Value::U8(0) | Value::U16(0) | Value::U32(0) | Value::U64(0) | Value::U128(0) | Value::Usize(0) => true,
        Value::F32(f) if *f == 0.0 => true,
        Value::F64(f) if *f == 0.0 => true,
        _ => false,
    };

    if is_zero {
        return Err(EvalError::DivisionByZero { span });
    }

    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a.checked_div(b).map(Value::I8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a.checked_div(b).map(Value::I16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a.checked_div(b).map(Value::I32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a.checked_div(b).map(Value::I64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a.checked_div(b).map(Value::I128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a.checked_div(b).map(Value::Isize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a.checked_div(b).map(Value::U8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a.checked_div(b).map(Value::U16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a.checked_div(b).map(Value::U32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a.checked_div(b).map(Value::U64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a.checked_div(b).map(Value::U128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a.checked_div(b).map(Value::Usize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a / b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a / b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "/".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_rem(left: Value, right: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    // Check for division by zero (remainder by zero)
    let is_zero = match &right {
        Value::I8(0) | Value::I16(0) | Value::I32(0) | Value::I64(0) | Value::I128(0) | Value::Isize(0) => true,
        Value::U8(0) | Value::U16(0) | Value::U32(0) | Value::U64(0) | Value::U128(0) | Value::Usize(0) => true,
        _ => false,
    };

    if is_zero {
        return Err(EvalError::DivisionByZero { span });
    }

    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a.checked_rem(b).map(Value::I8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a.checked_rem(b).map(Value::I16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a.checked_rem(b).map(Value::I32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a.checked_rem(b).map(Value::I64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a.checked_rem(b).map(Value::I128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a.checked_rem(b).map(Value::Isize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a.checked_rem(b).map(Value::U8).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a.checked_rem(b).map(Value::U16).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a.checked_rem(b).map(Value::U32).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a.checked_rem(b).map(Value::U64).ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a.checked_rem(b).map(Value::U128).ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a.checked_rem(b).map(Value::Usize).ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a % b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a % b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "%".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Comparison Operations
// ═══════════════════════════════════════════════════════════════════════

macro_rules! impl_comparison {
    ($name:ident, $op:tt, $op_str:expr) => {
        fn $name(
            left: Value,
            right: Value,
            span: Option<proc_macro2::Span>,
        ) -> Result<Value, EvalError> {
            match (&left, &right) {
                // Integers
                (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a $op b)),
                (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a $op b)),
                (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a $op b)),
                (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a $op b)),
                (Value::I128(a), Value::I128(b)) => Ok(Value::Bool(a $op b)),
                (Value::Isize(a), Value::Isize(b)) => Ok(Value::Bool(a $op b)),
                (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a $op b)),
                (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a $op b)),
                (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a $op b)),
                (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a $op b)),
                (Value::U128(a), Value::U128(b)) => Ok(Value::Bool(a $op b)),
                (Value::Usize(a), Value::Usize(b)) => Ok(Value::Bool(a $op b)),

                // Floats
                (Value::F32(a), Value::F32(b)) => Ok(Value::Bool(a $op b)),
                (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a $op b)),

                // Chars
                (Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a $op b)),

                // Strings
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a $op b)),

                _ => Err(EvalError::InvalidBinaryOperands {
                    op: $op_str.to_string(),
                    left_type: type_name(&left).to_string(),
                    right_type: type_name(&right).to_string(),
                    span,
                }),
            }
        }
    };
}

impl_comparison!(eval_lt, <, "<");
impl_comparison!(eval_le, <=, "<=");
impl_comparison!(eval_gt, >, ">");
impl_comparison!(eval_ge, >=, ">=");

// ═══════════════════════════════════════════════════════════════════════
// Bitwise Operations
// ═══════════════════════════════════════════════════════════════════════

macro_rules! impl_bitwise {
    ($name:ident, $op:tt, $op_str:expr) => {
        fn $name(
            left: Value,
            right: Value,
            span: Option<proc_macro2::Span>,
        ) -> Result<Value, EvalError> {
            match (left, right) {
                // Booleans (logical operation)
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a $op b)),

                // Integers
                (Value::I8(a), Value::I8(b)) => Ok(Value::I8(a $op b)),
                (Value::I16(a), Value::I16(b)) => Ok(Value::I16(a $op b)),
                (Value::I32(a), Value::I32(b)) => Ok(Value::I32(a $op b)),
                (Value::I64(a), Value::I64(b)) => Ok(Value::I64(a $op b)),
                (Value::I128(a), Value::I128(b)) => Ok(Value::I128(a $op b)),
                (Value::Isize(a), Value::Isize(b)) => Ok(Value::Isize(a $op b)),
                (Value::U8(a), Value::U8(b)) => Ok(Value::U8(a $op b)),
                (Value::U16(a), Value::U16(b)) => Ok(Value::U16(a $op b)),
                (Value::U32(a), Value::U32(b)) => Ok(Value::U32(a $op b)),
                (Value::U64(a), Value::U64(b)) => Ok(Value::U64(a $op b)),
                (Value::U128(a), Value::U128(b)) => Ok(Value::U128(a $op b)),
                (Value::Usize(a), Value::Usize(b)) => Ok(Value::Usize(a $op b)),

                (left, right) => Err(EvalError::InvalidBinaryOperands {
                    op: $op_str.to_string(),
                    left_type: type_name(&left).to_string(),
                    right_type: type_name(&right).to_string(),
                    span,
                }),
            }
        }
    };
}

impl_bitwise!(eval_bitand, &, "&");
impl_bitwise!(eval_bitor, |, "|");
impl_bitwise!(eval_bitxor, ^, "^");

fn eval_shl(left: Value, right: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    // Right side must be u32 for shift amount
    let shift = match &right {
        Value::I8(n) => *n as u32,
        Value::I16(n) => *n as u32,
        Value::I32(n) => *n as u32,
        Value::I64(n) => *n as u32,
        Value::U8(n) => *n as u32,
        Value::U16(n) => *n as u32,
        Value::U32(n) => *n,
        Value::U64(n) => *n as u32,
        Value::Usize(n) => *n as u32,
        _ => {
            return Err(EvalError::InvalidBinaryOperands {
                op: "<<".to_string(),
                left_type: type_name(&left).to_string(),
                right_type: type_name(&right).to_string(),
                span,
            });
        }
    };

    match left {
        Value::I8(a) => a.checked_shl(shift).map(Value::I8).ok_or(EvalError::IntegerOverflow { span }),
        Value::I16(a) => a.checked_shl(shift).map(Value::I16).ok_or(EvalError::IntegerOverflow { span }),
        Value::I32(a) => a.checked_shl(shift).map(Value::I32).ok_or(EvalError::IntegerOverflow { span }),
        Value::I64(a) => a.checked_shl(shift).map(Value::I64).ok_or(EvalError::IntegerOverflow { span }),
        Value::I128(a) => a.checked_shl(shift).map(Value::I128).ok_or(EvalError::IntegerOverflow { span }),
        Value::Isize(a) => a.checked_shl(shift).map(Value::Isize).ok_or(EvalError::IntegerOverflow { span }),
        Value::U8(a) => a.checked_shl(shift).map(Value::U8).ok_or(EvalError::IntegerOverflow { span }),
        Value::U16(a) => a.checked_shl(shift).map(Value::U16).ok_or(EvalError::IntegerOverflow { span }),
        Value::U32(a) => a.checked_shl(shift).map(Value::U32).ok_or(EvalError::IntegerOverflow { span }),
        Value::U64(a) => a.checked_shl(shift).map(Value::U64).ok_or(EvalError::IntegerOverflow { span }),
        Value::U128(a) => a.checked_shl(shift).map(Value::U128).ok_or(EvalError::IntegerOverflow { span }),
        Value::Usize(a) => a.checked_shl(shift).map(Value::Usize).ok_or(EvalError::IntegerOverflow { span }),
        _ => Err(EvalError::InvalidBinaryOperands {
            op: "<<".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_shr(left: Value, right: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    let shift = match &right {
        Value::I8(n) => *n as u32,
        Value::I16(n) => *n as u32,
        Value::I32(n) => *n as u32,
        Value::I64(n) => *n as u32,
        Value::U8(n) => *n as u32,
        Value::U16(n) => *n as u32,
        Value::U32(n) => *n,
        Value::U64(n) => *n as u32,
        Value::Usize(n) => *n as u32,
        _ => {
            return Err(EvalError::InvalidBinaryOperands {
                op: ">>".to_string(),
                left_type: type_name(&left).to_string(),
                right_type: type_name(&right).to_string(),
                span,
            });
        }
    };

    match left {
        Value::I8(a) => a.checked_shr(shift).map(Value::I8).ok_or(EvalError::IntegerOverflow { span }),
        Value::I16(a) => a.checked_shr(shift).map(Value::I16).ok_or(EvalError::IntegerOverflow { span }),
        Value::I32(a) => a.checked_shr(shift).map(Value::I32).ok_or(EvalError::IntegerOverflow { span }),
        Value::I64(a) => a.checked_shr(shift).map(Value::I64).ok_or(EvalError::IntegerOverflow { span }),
        Value::I128(a) => a.checked_shr(shift).map(Value::I128).ok_or(EvalError::IntegerOverflow { span }),
        Value::Isize(a) => a.checked_shr(shift).map(Value::Isize).ok_or(EvalError::IntegerOverflow { span }),
        Value::U8(a) => a.checked_shr(shift).map(Value::U8).ok_or(EvalError::IntegerOverflow { span }),
        Value::U16(a) => a.checked_shr(shift).map(Value::U16).ok_or(EvalError::IntegerOverflow { span }),
        Value::U32(a) => a.checked_shr(shift).map(Value::U32).ok_or(EvalError::IntegerOverflow { span }),
        Value::U64(a) => a.checked_shr(shift).map(Value::U64).ok_or(EvalError::IntegerOverflow { span }),
        Value::U128(a) => a.checked_shr(shift).map(Value::U128).ok_or(EvalError::IntegerOverflow { span }),
        Value::Usize(a) => a.checked_shr(shift).map(Value::Usize).ok_or(EvalError::IntegerOverflow { span }),
        _ => Err(EvalError::InvalidBinaryOperands {
            op: ">>".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}
```

---

## Update lib.rs

### src/lib.rs

```rust
pub mod value;
pub mod environment;
pub mod context;
pub mod error;
pub mod eval;

pub use value::{
    Value,
    StructValue,
    EnumValue,
    EnumData,
    FunctionValue,
    ClosureValue,
    BuiltinFn,
    CompiledFn,
    ValueRef,
    ValueRefMut,
    HashableValue,
};

pub use environment::{Environment, Binding, BindingMode, ScopeGuard};
pub use context::EvalContext;
pub use error::{TreebeardError, EnvironmentError, EvalError};
pub use eval::{Evaluate, eval_expr};
```

---

## Test Cases

### tests/eval_tests.rs

```rust
use treebeard_core::*;

// Helper to parse and evaluate an expression
fn eval(src: &str) -> Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    expr.eval(&mut env, &ctx)
}

// Helper with pre-defined environment
fn eval_with_env(src: &str, env: &mut Environment) -> Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let ctx = EvalContext::default();
    expr.eval(env, &ctx)
}

// ═══════════════════════════════════════════════════════════════════════
// Literal Evaluation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_lit_integer() {
    assert_eq!(eval("42").unwrap(), Value::I64(42));
    assert_eq!(eval("0").unwrap(), Value::I64(0));
    assert_eq!(eval("-1").unwrap(), Value::I64(-1));
}

#[test]
fn test_eval_lit_integer_suffixes() {
    assert_eq!(eval("42i8").unwrap(), Value::I8(42));
    assert_eq!(eval("42i16").unwrap(), Value::I16(42));
    assert_eq!(eval("42i32").unwrap(), Value::I32(42));
    assert_eq!(eval("42i64").unwrap(), Value::I64(42));
    assert_eq!(eval("42u8").unwrap(), Value::U8(42));
    assert_eq!(eval("42u32").unwrap(), Value::U32(42));
    assert_eq!(eval("42usize").unwrap(), Value::Usize(42));
}

#[test]
fn test_eval_lit_float() {
    assert_eq!(eval("3.14").unwrap(), Value::F64(3.14));
    assert_eq!(eval("3.14f32").unwrap(), Value::F32(3.14));
    assert_eq!(eval("3.14f64").unwrap(), Value::F64(3.14));
}

#[test]
fn test_eval_lit_bool() {
    assert_eq!(eval("true").unwrap(), Value::Bool(true));
    assert_eq!(eval("false").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_lit_char() {
    assert_eq!(eval("'a'").unwrap(), Value::Char('a'));
    assert_eq!(eval("'\\n'").unwrap(), Value::Char('\n'));
}

#[test]
fn test_eval_lit_string() {
    assert_eq!(eval(r#""hello""#).unwrap(), Value::string("hello"));
    assert_eq!(eval(r#""hello\nworld""#).unwrap(), Value::string("hello\nworld"));
}

#[test]
fn test_eval_lit_byte() {
    assert_eq!(eval("b'a'").unwrap(), Value::U8(b'a'));
}

// ═══════════════════════════════════════════════════════════════════════
// Path Evaluation (Variable Lookup)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_path_defined() {
    let mut env = Environment::new();
    env.define("x", Value::I64(42));

    assert_eq!(eval_with_env("x", &mut env).unwrap(), Value::I64(42));
}

#[test]
fn test_eval_path_undefined() {
    let result = eval("undefined_var");
    assert!(matches!(result, Err(EvalError::UndefinedVariable { .. })));
}

#[test]
fn test_eval_path_shadowing() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));
    env.push_frame();
    env.define("x", Value::I64(2));

    assert_eq!(eval_with_env("x", &mut env).unwrap(), Value::I64(2));
}

// ═══════════════════════════════════════════════════════════════════════
// Unary Operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_unary_neg() {
    assert_eq!(eval("-42").unwrap(), Value::I64(-42));
    assert_eq!(eval("-3.14").unwrap(), Value::F64(-3.14));
}

#[test]
fn test_eval_unary_neg_overflow() {
    // i8::MIN cannot be negated
    let result = eval("-(-128i8)");
    assert!(matches!(result, Err(EvalError::IntegerOverflow { .. })));
}

#[test]
fn test_eval_unary_not_bool() {
    assert_eq!(eval("!true").unwrap(), Value::Bool(false));
    assert_eq!(eval("!false").unwrap(), Value::Bool(true));
}

#[test]
fn test_eval_unary_not_bitwise() {
    assert_eq!(eval("!0u8").unwrap(), Value::U8(255));
    assert_eq!(eval("!0i32").unwrap(), Value::I32(-1));
}

#[test]
fn test_eval_unary_invalid() {
    let result = eval("-true");
    assert!(matches!(result, Err(EvalError::InvalidUnaryOperand { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Arithmetic
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_add() {
    assert_eq!(eval("1 + 2").unwrap(), Value::I64(3));
    assert_eq!(eval("1.5 + 2.5").unwrap(), Value::F64(4.0));
}

#[test]
fn test_eval_binary_add_string() {
    assert_eq!(
        eval(r#""hello" + " world""#).unwrap(),
        Value::string("hello world")
    );
}

#[test]
fn test_eval_binary_sub() {
    assert_eq!(eval("5 - 3").unwrap(), Value::I64(2));
    assert_eq!(eval("3.5 - 1.5").unwrap(), Value::F64(2.0));
}

#[test]
fn test_eval_binary_mul() {
    assert_eq!(eval("3 * 4").unwrap(), Value::I64(12));
    assert_eq!(eval("2.0 * 3.0").unwrap(), Value::F64(6.0));
}

#[test]
fn test_eval_binary_div() {
    assert_eq!(eval("10 / 3").unwrap(), Value::I64(3)); // Integer division
    assert_eq!(eval("10.0 / 4.0").unwrap(), Value::F64(2.5));
}

#[test]
fn test_eval_binary_div_by_zero() {
    let result = eval("1 / 0");
    assert!(matches!(result, Err(EvalError::DivisionByZero { .. })));
}

#[test]
fn test_eval_binary_rem() {
    assert_eq!(eval("10 % 3").unwrap(), Value::I64(1));
    assert_eq!(eval("10 % 5").unwrap(), Value::I64(0));
}

#[test]
fn test_eval_binary_overflow() {
    let result = eval("127i8 + 1i8");
    assert!(matches!(result, Err(EvalError::IntegerOverflow { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Comparison
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_eq() {
    assert_eq!(eval("1 == 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 == 2").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_ne() {
    assert_eq!(eval("1 != 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 != 1").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_lt() {
    assert_eq!(eval("1 < 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("2 < 1").unwrap(), Value::Bool(false));
    assert_eq!(eval("1 < 1").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_le() {
    assert_eq!(eval("1 <= 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 <= 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("2 <= 1").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_gt() {
    assert_eq!(eval("2 > 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 > 2").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_ge() {
    assert_eq!(eval("2 >= 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("2 >= 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 >= 2").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_string_comparison() {
    assert_eq!(eval(r#""a" < "b""#).unwrap(), Value::Bool(true));
    assert_eq!(eval(r#""abc" == "abc""#).unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Logical (Short-Circuit)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_and() {
    assert_eq!(eval("true && true").unwrap(), Value::Bool(true));
    assert_eq!(eval("true && false").unwrap(), Value::Bool(false));
    assert_eq!(eval("false && true").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_or() {
    assert_eq!(eval("true || false").unwrap(), Value::Bool(true));
    assert_eq!(eval("false || true").unwrap(), Value::Bool(true));
    assert_eq!(eval("false || false").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_and_short_circuit() {
    // This would fail if not short-circuiting (undefined_var doesn't exist)
    let mut env = Environment::new();
    let result = eval_with_env("false && undefined_var", &mut env);
    assert_eq!(result.unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_or_short_circuit() {
    let mut env = Environment::new();
    let result = eval_with_env("true || undefined_var", &mut env);
    assert_eq!(result.unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Bitwise
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_bitand() {
    assert_eq!(eval("0b1100u8 & 0b1010u8").unwrap(), Value::U8(0b1000));
}

#[test]
fn test_eval_binary_bitor() {
    assert_eq!(eval("0b1100u8 | 0b1010u8").unwrap(), Value::U8(0b1110));
}

#[test]
fn test_eval_binary_bitxor() {
    assert_eq!(eval("0b1100u8 ^ 0b1010u8").unwrap(), Value::U8(0b0110));
}

#[test]
fn test_eval_binary_shl() {
    assert_eq!(eval("1 << 4").unwrap(), Value::I64(16));
    assert_eq!(eval("1u8 << 7u32").unwrap(), Value::U8(128));
}

#[test]
fn test_eval_binary_shr() {
    assert_eq!(eval("16 >> 2").unwrap(), Value::I64(4));
    assert_eq!(eval("128u8 >> 7u32").unwrap(), Value::U8(1));
}

// ═══════════════════════════════════════════════════════════════════════
// Type Mismatches
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_type_mismatch() {
    let result = eval("1i32 + 1i64");
    assert!(matches!(result, Err(EvalError::InvalidBinaryOperands { .. })));
}

#[test]
fn test_eval_binary_invalid_types() {
    let result = eval("true + false");
    assert!(matches!(result, Err(EvalError::InvalidBinaryOperands { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Parentheses and Precedence
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_parentheses() {
    assert_eq!(eval("(1 + 2) * 3").unwrap(), Value::I64(9));
    assert_eq!(eval("1 + (2 * 3)").unwrap(), Value::I64(7));
}

#[test]
fn test_eval_nested_expressions() {
    assert_eq!(eval("((1 + 2) * (3 + 4))").unwrap(), Value::I64(21));
}

// ═══════════════════════════════════════════════════════════════════════
// Complex Expressions with Variables
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_expression_with_vars() {
    let mut env = Environment::new();
    env.define("x", Value::I64(10));
    env.define("y", Value::I64(20));

    assert_eq!(eval_with_env("x + y", &mut env).unwrap(), Value::I64(30));
    assert_eq!(eval_with_env("x * y + 5", &mut env).unwrap(), Value::I64(205));
    assert_eq!(eval_with_env("x < y", &mut env).unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Interruption
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_interrupted() {
    let expr: syn::Expr = syn::parse_str("1 + 2").unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();

    // Set interrupt before evaluation
    ctx.interrupt();

    let result = expr.eval(&mut env, &ctx);
    assert!(matches!(result, Err(EvalError::Interrupted)));
}
```

---

## Completion Checklist

- [ ] Create `src/context.rs` with `EvalContext`
- [ ] Extend `src/error.rs` with `EvalError` and `type_name` helper
- [ ] Create `src/eval/mod.rs` with `Evaluate` trait and `syn::Expr` dispatcher
- [ ] Create `src/eval/literal.rs` with `syn::ExprLit` evaluation
- [ ] Create `src/eval/path.rs` with `syn::ExprPath` evaluation
- [ ] Create `src/eval/unary.rs` with `syn::ExprUnary` evaluation
- [ ] Create `src/eval/binary.rs` with `syn::ExprBinary` evaluation
- [ ] Implement all arithmetic operators with overflow checking
- [ ] Implement all comparison operators
- [ ] Implement short-circuit logical operators (`&&`, `||`)
- [ ] Implement bitwise operators
- [ ] Handle parenthesized expressions
- [ ] Update `lib.rs` exports
- [ ] All tests passing

---

## Design Notes

### Why Checked Arithmetic?

Rust's default behavior is to panic on overflow in debug builds. We use checked arithmetic (`checked_add`, etc.) to return proper errors instead of panicking, which is essential for an interpreter.

### Why Short-Circuit Evaluation?

`&&` and `||` must short-circuit to match Rust semantics. `false && expensive()` should not evaluate `expensive()`. This also enables useful patterns like `x != 0 && 10 / x > 2`.

### Why Require Same Types?

Rust doesn't implicitly coerce numeric types. `1i32 + 1i64` is a type error. We follow Rust semantics strictly, requiring explicit type suffixes for mixed-type operations.

### Why Return EvalError Instead of Panic?

An interpreter should never panic on user code. All errors should be catchable and reportable with source locations.

---

## Next Stage

**Stage 1.4: Control Flow** — Implement `if`/`else`, `match` expressions, `loop`/`while`/`for`, and `break`/`continue`.
