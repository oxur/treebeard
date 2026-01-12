# Stage 1.4: Control Flow

**Phase:** 1 - Core Evaluator
**Stage:** 1.4
**Prerequisites:** Stage 1.1 (Value), Stage 1.2 (Environment), Stage 1.3 (Basic Expressions)
**Estimated effort:** 3-4 days

---

## Objective

Implement control flow expressions: `if`/`else`, `match`, loops (`loop`/`while`/`for`), and loop control (`break`/`continue`). This enables conditional execution and iteration in evaluated code.

---

## Overview

Control flow in Rust expressions:

| Type | Example | Notes |
|------|---------|-------|
| `syn::ExprIf` | `if x > 0 { 1 } else { 0 }` | Condition must be bool; both arms same type |
| `syn::ExprMatch` | `match x { 1 => "one", _ => "other" }` | Pattern matching with guards |
| `syn::ExprLoop` | `loop { break 42; }` | Infinite loop; value from `break` |
| `syn::ExprWhile` | `while x > 0 { x -= 1 }` | Conditional loop; returns `()` |
| `syn::ExprForLoop` | `for i in 0..10 { ... }` | Iterator loop; returns `()` |
| `syn::ExprBreak` | `break 42` or `break` | Exit loop with optional value |
| `syn::ExprContinue` | `continue` | Skip to next iteration |

**Key challenge:** `break` and `continue` are non-local control flow. We handle them using a `ControlFlow` enum that propagates up the call stack until caught by a loop.

---

## File Structure

```
treebeard/src/
├── lib.rs              # Update exports
├── eval/
│   ├── mod.rs          # Update dispatcher
│   ├── control.rs      # ← New: ControlFlow enum
│   ├── if_expr.rs      # ← New: syn::ExprIf
│   ├── match_expr.rs   # ← New: syn::ExprMatch
│   ├── loops.rs        # ← New: loop/while/for
│   └── pattern.rs      # ← New: Pattern matching
└── ...
```

---

## Control Flow Mechanism

### src/eval/control.rs

```rust
use crate::Value;

/// Control flow signal for non-local jumps.
///
/// When `break` or `continue` is evaluated, it doesn't return a normal
/// `Result<Value, EvalError>`. Instead, it returns an `Err(EvalError::ControlFlow(...))`
/// that propagates up until caught by the enclosing loop.
#[derive(Debug, Clone)]
pub enum ControlFlow {
    /// Break out of a loop, optionally with a value.
    /// The Option<String> is the loop label (e.g., `break 'outer`).
    Break {
        value: Value,
        label: Option<String>,
    },

    /// Continue to next iteration of a loop.
    Continue {
        label: Option<String>,
    },

    /// Return from a function with a value.
    /// (Used in Stage 1.5, but defined here for completeness)
    Return {
        value: Value,
    },
}

impl ControlFlow {
    /// Create a break with a value.
    pub fn break_with(value: Value) -> Self {
        ControlFlow::Break { value, label: None }
    }

    /// Create a break with unit value.
    pub fn break_unit() -> Self {
        ControlFlow::Break {
            value: Value::Unit,
            label: None,
        }
    }

    /// Create a labeled break.
    pub fn break_labeled(value: Value, label: String) -> Self {
        ControlFlow::Break {
            value,
            label: Some(label),
        }
    }

    /// Create a continue.
    pub fn continue_loop() -> Self {
        ControlFlow::Continue { label: None }
    }

    /// Create a labeled continue.
    pub fn continue_labeled(label: String) -> Self {
        ControlFlow::Continue { label: Some(label) }
    }

    /// Create a return.
    pub fn return_value(value: Value) -> Self {
        ControlFlow::Return { value }
    }

    /// Check if this control flow matches a label.
    /// None label matches any loop, Some(l) matches only that label.
    pub fn matches_label(&self, loop_label: Option<&str>) -> bool {
        match self {
            ControlFlow::Break { label, .. } | ControlFlow::Continue { label } => {
                match (label, loop_label) {
                    (None, _) => true,              // Unlabeled matches any
                    (Some(l), Some(ll)) => l == ll, // Labels must match
                    (Some(_), None) => false,       // Labeled doesn't match unlabeled
                }
            }
            ControlFlow::Return { .. } => false, // Return never matches a loop
        }
    }
}
```

### Extend src/error.rs

Add `ControlFlow` variant to `EvalError`:

```rust
use crate::eval::control::ControlFlow;

#[derive(Error, Debug, Clone)]
pub enum EvalError {
    // ... existing variants ...

    /// Control flow (break/continue/return) - not really an error,
    /// but uses the error path for propagation.
    #[error("control flow")]
    ControlFlow(ControlFlow),

    /// Break outside of loop.
    #[error("`break` outside of loop")]
    BreakOutsideLoop { span: Option<Span> },

    /// Continue outside of loop.
    #[error("`continue` outside of loop")]
    ContinueOutsideLoop { span: Option<Span> },

    /// Return outside of function.
    #[error("`return` outside of function")]
    ReturnOutsideFunction { span: Option<Span> },

    /// Non-exhaustive match.
    #[error("non-exhaustive match: `{value}` not covered")]
    NonExhaustiveMatch {
        value: String,
        span: Option<Span>,
    },

    /// Refutable pattern in irrefutable context.
    #[error("refutable pattern in local binding")]
    RefutablePattern {
        pattern: String,
        span: Option<Span>,
    },
}

impl EvalError {
    /// Check if this is a control flow "error" (not a real error).
    pub fn is_control_flow(&self) -> bool {
        matches!(self, EvalError::ControlFlow(_))
    }

    /// Extract control flow if this is one.
    pub fn into_control_flow(self) -> Option<ControlFlow> {
        match self {
            EvalError::ControlFlow(cf) => Some(cf),
            _ => None,
        }
    }
}
```

---

## If Expression

### src/eval/if_expr.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;

impl Evaluate for syn::ExprIf {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate condition
        let cond = self.cond.eval(env, ctx)?;

        // Condition must be boolean
        let cond_bool = match cond {
            Value::Bool(b) => b,
            other => {
                return Err(EvalError::TypeError {
                    message: format!(
                        "expected `bool` in if condition, found `{}`",
                        crate::error::type_name(&other)
                    ),
                    span: expr_span(&self.cond),
                });
            }
        };

        if cond_bool {
            // Evaluate then branch
            eval_block(&self.then_branch, env, ctx)
        } else if let Some((_, else_branch)) = &self.else_branch {
            // Evaluate else branch
            match else_branch.as_ref() {
                syn::Expr::Block(block) => eval_block(&block.block, env, ctx),
                syn::Expr::If(else_if) => else_if.eval(env, ctx),
                other => other.eval(env, ctx),
            }
        } else {
            // No else branch, return unit
            Ok(Value::Unit)
        }
    }
}

/// Evaluate a block, returning the value of the last expression.
pub fn eval_block(
    block: &syn::Block,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Push a new scope for the block
    env.push_frame();

    let result = eval_block_inner(block, env, ctx);

    // Pop the scope (even on error)
    env.pop_frame();

    result
}

fn eval_block_inner(
    block: &syn::Block,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let mut last_value = Value::Unit;

    for stmt in &block.stmts {
        last_value = eval_stmt(stmt, env, ctx)?;
    }

    Ok(last_value)
}

/// Evaluate a statement (placeholder - full impl in Stage 1.6).
fn eval_stmt(
    stmt: &syn::Stmt,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    match stmt {
        syn::Stmt::Expr(expr, None) => {
            // Expression without semicolon - its value is the block's value
            expr.eval(env, ctx)
        }
        syn::Stmt::Expr(expr, Some(_)) => {
            // Expression with semicolon - evaluate for side effects, return unit
            expr.eval(env, ctx)?;
            Ok(Value::Unit)
        }
        syn::Stmt::Local(_) => {
            // Let binding - implemented in Stage 1.6
            Err(EvalError::UnsupportedExpr {
                kind: "let binding (not yet implemented)".to_string(),
                span: None,
            })
        }
        syn::Stmt::Item(_) => {
            // Item in block - implemented in Stage 1.5/1.6
            Err(EvalError::UnsupportedExpr {
                kind: "item in block (not yet implemented)".to_string(),
                span: None,
            })
        }
        syn::Stmt::Macro(_) => {
            Err(EvalError::UnsupportedExpr {
                kind: "macro statement".to_string(),
                span: None,
            })
        }
    }
}

fn expr_span(expr: &syn::Expr) -> Option<proc_macro2::Span> {
    use quote::ToTokens;
    expr.to_token_stream()
        .into_iter()
        .next()
        .map(|t| t.span())
}
```

---

## Pattern Matching

### src/eval/pattern.rs

```rust
use crate::{Value, Environment, EvalError};
use proc_macro2::Span;

/// Result of pattern matching: bindings to add to environment.
pub type MatchBindings = Vec<(String, Value, bool)>; // (name, value, mutable)

/// Match a value against a pattern.
///
/// Returns `Ok(Some(bindings))` if the pattern matches,
/// `Ok(None)` if it doesn't match,
/// `Err(...)` if there's an error.
pub fn match_pattern(
    pattern: &syn::Pat,
    value: &Value,
    _span: Option<Span>,
) -> Result<Option<MatchBindings>, EvalError> {
    match pattern {
        // Wildcard: matches anything, no bindings
        syn::Pat::Wild(_) => Ok(Some(vec![])),

        // Identifier: matches anything, binds the value
        syn::Pat::Ident(pat_ident) => {
            let name = pat_ident.ident.to_string();
            let mutable = pat_ident.mutability.is_some();

            // Check for @ pattern (e.g., `x @ 1..=5`)
            if let Some((_, subpat)) = &pat_ident.subpat {
                // Must also match the subpattern
                if let Some(mut bindings) = match_pattern(subpat, value, None)? {
                    bindings.push((name, value.clone(), mutable));
                    Ok(Some(bindings))
                } else {
                    Ok(None)
                }
            } else {
                Ok(Some(vec![(name, value.clone(), mutable)]))
            }
        }

        // Literal pattern: matches exact value
        syn::Pat::Lit(pat_lit) => {
            let lit_value = crate::eval::literal::eval_lit(&extract_lit(&pat_lit.lit)?)?;
            if value == &lit_value {
                Ok(Some(vec![]))
            } else {
                Ok(None)
            }
        }

        // Or pattern: try each alternative
        syn::Pat::Or(pat_or) => {
            for case in &pat_or.cases {
                if let Some(bindings) = match_pattern(case, value, None)? {
                    return Ok(Some(bindings));
                }
            }
            Ok(None)
        }

        // Tuple pattern: match each element
        syn::Pat::Tuple(pat_tuple) => match value {
            Value::Tuple(elements) => {
                if pat_tuple.elems.len() != elements.len() {
                    return Ok(None);
                }
                let mut all_bindings = vec![];
                for (pat, val) in pat_tuple.elems.iter().zip(elements.iter()) {
                    if let Some(bindings) = match_pattern(pat, val, None)? {
                        all_bindings.extend(bindings);
                    } else {
                        return Ok(None);
                    }
                }
                Ok(Some(all_bindings))
            }
            _ => Ok(None),
        },

        // Struct pattern: match fields
        syn::Pat::Struct(pat_struct) => match value {
            Value::Struct(s) => {
                // Check type name matches (simplified - just check last segment)
                let pat_type = pat_struct
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                if s.type_name != pat_type {
                    return Ok(None);
                }

                let mut all_bindings = vec![];
                for field_pat in &pat_struct.fields {
                    let field_name = match &field_pat.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(idx) => idx.index.to_string(),
                    };

                    let field_value = s.fields.get(&field_name).cloned().unwrap_or(Value::Unit);

                    if let Some(bindings) = match_pattern(&field_pat.pat, &field_value, None)? {
                        all_bindings.extend(bindings);
                    } else {
                        return Ok(None);
                    }
                }

                // Handle `..` rest pattern
                // (We don't need to do anything special - just ignore unmatched fields)

                Ok(Some(all_bindings))
            }
            _ => Ok(None),
        },

        // TupleStruct pattern (e.g., Some(x))
        syn::Pat::TupleStruct(pat_ts) => match value {
            Value::Enum(e) => {
                // Check variant name matches
                let pat_variant = pat_ts
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                if e.variant != pat_variant {
                    return Ok(None);
                }

                // Match inner data
                match &e.data {
                    crate::EnumData::Tuple(elements) => {
                        if pat_ts.elems.len() != elements.len() {
                            return Ok(None);
                        }
                        let mut all_bindings = vec![];
                        for (pat, val) in pat_ts.elems.iter().zip(elements.iter()) {
                            if let Some(bindings) = match_pattern(pat, val, None)? {
                                all_bindings.extend(bindings);
                            } else {
                                return Ok(None);
                            }
                        }
                        Ok(Some(all_bindings))
                    }
                    _ => Ok(None),
                }
            }
            Value::Option(opt) => {
                let pat_variant = pat_ts
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                match (pat_variant.as_str(), opt.as_ref()) {
                    ("Some", Some(inner)) if pat_ts.elems.len() == 1 => {
                        match_pattern(&pat_ts.elems[0], inner, None)
                    }
                    ("None", None) if pat_ts.elems.is_empty() => Ok(Some(vec![])),
                    _ => Ok(None),
                }
            }
            Value::Result(res) => {
                let pat_variant = pat_ts
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                match (pat_variant.as_str(), res.as_ref()) {
                    ("Ok", Ok(inner)) if pat_ts.elems.len() == 1 => {
                        match_pattern(&pat_ts.elems[0], inner, None)
                    }
                    ("Err", Err(inner)) if pat_ts.elems.len() == 1 => {
                        match_pattern(&pat_ts.elems[0], inner, None)
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        },

        // Path pattern (e.g., None, MyEnum::Variant)
        syn::Pat::Path(pat_path) => {
            let variant = pat_path
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();

            match value {
                Value::Option(None) if variant == "None" => Ok(Some(vec![])),
                Value::Enum(e) if e.variant == variant => {
                    match &e.data {
                        crate::EnumData::Unit => Ok(Some(vec![])),
                        _ => Ok(None), // Has data but pattern doesn't expect it
                    }
                }
                _ => Ok(None),
            }
        }

        // Range pattern (e.g., 1..=5)
        syn::Pat::Range(pat_range) => {
            // Evaluate bounds
            let start = pat_range
                .start
                .as_ref()
                .map(|e| eval_const_expr(e))
                .transpose()?;
            let end = pat_range
                .end
                .as_ref()
                .map(|e| eval_const_expr(e))
                .transpose()?;

            let in_range = match (start, end, &pat_range.limits) {
                (Some(s), Some(e), syn::RangeLimits::Closed(_)) => {
                    value_in_range_inclusive(value, &s, &e)
                }
                (Some(s), Some(e), syn::RangeLimits::HalfOpen(_)) => {
                    value_in_range_exclusive(value, &s, &e)
                }
                _ => {
                    return Err(EvalError::UnsupportedExpr {
                        kind: "unbounded range pattern".to_string(),
                        span: None,
                    });
                }
            };

            if in_range {
                Ok(Some(vec![]))
            } else {
                Ok(None)
            }
        }

        // Reference pattern
        syn::Pat::Reference(pat_ref) => {
            // For now, just match the inner pattern against the value
            // (We're not tracking references strictly yet)
            match_pattern(&pat_ref.pat, value, None)
        }

        // Rest pattern (..) - handled by parent patterns
        syn::Pat::Rest(_) => Ok(Some(vec![])),

        // Slice pattern
        syn::Pat::Slice(pat_slice) => match value {
            Value::Vec(elements) | Value::Array(elements) => {
                // Check for rest pattern
                let has_rest = pat_slice.elems.iter().any(|p| matches!(p, syn::Pat::Rest(_)));

                if has_rest {
                    // Complex slice matching with ..
                    match_slice_with_rest(&pat_slice.elems, elements)
                } else {
                    // Simple: exact length match
                    if pat_slice.elems.len() != elements.len() {
                        return Ok(None);
                    }
                    let mut all_bindings = vec![];
                    for (pat, val) in pat_slice.elems.iter().zip(elements.iter()) {
                        if let Some(bindings) = match_pattern(pat, val, None)? {
                            all_bindings.extend(bindings);
                        } else {
                            return Ok(None);
                        }
                    }
                    Ok(Some(all_bindings))
                }
            }
            _ => Ok(None),
        },

        // Const pattern (named constant)
        syn::Pat::Const(_) => Err(EvalError::UnsupportedExpr {
            kind: "const pattern".to_string(),
            span: None,
        }),

        // Macro pattern
        syn::Pat::Macro(_) => Err(EvalError::UnsupportedExpr {
            kind: "macro pattern".to_string(),
            span: None,
        }),

        // Paren pattern - unwrap
        syn::Pat::Paren(pat) => match_pattern(&pat.pat, value, None),

        // Type pattern (x: Type)
        syn::Pat::Type(pat_type) => {
            // Just match the inner pattern, ignore type annotation
            match_pattern(&pat_type.pat, value, None)
        }

        // Verbatim pattern
        syn::Pat::Verbatim(_) => Err(EvalError::UnsupportedExpr {
            kind: "verbatim pattern".to_string(),
            span: None,
        }),

        _ => Err(EvalError::UnsupportedExpr {
            kind: "unknown pattern".to_string(),
            span: None,
        }),
    }
}

/// Extract a syn::Lit from a syn::Expr (for literal patterns).
fn extract_lit(expr: &syn::Expr) -> Result<syn::Lit, EvalError> {
    match expr {
        syn::Expr::Lit(lit) => Ok(lit.lit.clone()),
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            // Handle negative literals like -1
            if let syn::Expr::Lit(lit) = unary.expr.as_ref() {
                Ok(lit.lit.clone())
            } else {
                Err(EvalError::UnsupportedExpr {
                    kind: "non-literal in pattern".to_string(),
                    span: None,
                })
            }
        }
        _ => Err(EvalError::UnsupportedExpr {
            kind: "non-literal in pattern".to_string(),
            span: None,
        }),
    }
}

/// Evaluate a constant expression (for range patterns).
fn eval_const_expr(expr: &syn::Expr) -> Result<Value, EvalError> {
    // Only handle literals and negated literals for now
    match expr {
        syn::Expr::Lit(lit) => crate::eval::literal::eval_lit(&lit.lit),
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            let inner = eval_const_expr(&unary.expr)?;
            crate::eval::unary::eval_neg(inner, None)
        }
        _ => Err(EvalError::UnsupportedExpr {
            kind: "non-constant in range pattern".to_string(),
            span: None,
        }),
    }
}

/// Check if value is in inclusive range [start, end].
fn value_in_range_inclusive(value: &Value, start: &Value, end: &Value) -> bool {
    match (value, start, end) {
        (Value::I64(v), Value::I64(s), Value::I64(e)) => *v >= *s && *v <= *e,
        (Value::I32(v), Value::I32(s), Value::I32(e)) => *v >= *s && *v <= *e,
        (Value::U64(v), Value::U64(s), Value::U64(e)) => *v >= *s && *v <= *e,
        (Value::U32(v), Value::U32(s), Value::U32(e)) => *v >= *s && *v <= *e,
        (Value::Char(v), Value::Char(s), Value::Char(e)) => *v >= *s && *v <= *e,
        _ => false,
    }
}

/// Check if value is in exclusive range [start, end).
fn value_in_range_exclusive(value: &Value, start: &Value, end: &Value) -> bool {
    match (value, start, end) {
        (Value::I64(v), Value::I64(s), Value::I64(e)) => *v >= *s && *v < *e,
        (Value::I32(v), Value::I32(s), Value::I32(e)) => *v >= *s && *v < *e,
        (Value::U64(v), Value::U64(s), Value::U64(e)) => *v >= *s && *v < *e,
        (Value::U32(v), Value::U32(s), Value::U32(e)) => *v >= *s && *v < *e,
        (Value::Char(v), Value::Char(s), Value::Char(e)) => *v >= *s && *v < *e,
        _ => false,
    }
}

/// Match a slice pattern with rest (..).
fn match_slice_with_rest(
    patterns: &syn::punctuated::Punctuated<syn::Pat, syn::Token![,]>,
    elements: &[Value],
) -> Result<Option<MatchBindings>, EvalError> {
    // Find the rest pattern position
    let rest_pos = patterns
        .iter()
        .position(|p| matches!(p, syn::Pat::Rest(_)))
        .unwrap();

    let before_rest = &patterns.iter().collect::<Vec<_>>()[..rest_pos];
    let after_rest = &patterns.iter().collect::<Vec<_>>()[rest_pos + 1..];

    // Need at least enough elements for patterns before and after rest
    if elements.len() < before_rest.len() + after_rest.len() {
        return Ok(None);
    }

    let mut all_bindings = vec![];

    // Match patterns before rest
    for (pat, val) in before_rest.iter().zip(elements.iter()) {
        if let Some(bindings) = match_pattern(pat, val, None)? {
            all_bindings.extend(bindings);
        } else {
            return Ok(None);
        }
    }

    // Match patterns after rest (from the end)
    let after_start = elements.len() - after_rest.len();
    for (pat, val) in after_rest.iter().zip(elements[after_start..].iter()) {
        if let Some(bindings) = match_pattern(pat, val, None)? {
            all_bindings.extend(bindings);
        } else {
            return Ok(None);
        }
    }

    Ok(Some(all_bindings))
}

/// Apply match bindings to the environment.
pub fn apply_bindings(env: &mut Environment, bindings: MatchBindings) {
    for (name, value, mutable) in bindings {
        if mutable {
            env.define_with_mode(&name, value, crate::BindingMode::Mutable);
        } else {
            env.define(&name, value);
        }
    }
}
```

---

## Match Expression

### src/eval/match_expr.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;
use super::pattern::{match_pattern, apply_bindings};
use super::if_expr::eval_block;

impl Evaluate for syn::ExprMatch {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate the scrutinee
        let scrutinee = self.expr.eval(env, ctx)?;

        // Try each arm
        for arm in &self.arms {
            // Check if pattern matches
            if let Some(bindings) = match_pattern(&arm.pat, &scrutinee, None)? {
                // Check guard if present
                let guard_passes = if let Some((_, guard)) = &arm.guard {
                    // Temporarily add bindings for guard evaluation
                    env.push_frame();
                    apply_bindings(env, bindings.clone());
                    let guard_result = guard.eval(env, ctx);
                    env.pop_frame();

                    match guard_result? {
                        Value::Bool(b) => b,
                        other => {
                            return Err(EvalError::TypeError {
                                message: format!(
                                    "expected `bool` in match guard, found `{}`",
                                    crate::error::type_name(&other)
                                ),
                                span: None,
                            });
                        }
                    }
                } else {
                    true
                };

                if guard_passes {
                    // Pattern matches and guard passes - evaluate body
                    env.push_frame();
                    apply_bindings(env, bindings);
                    let result = arm.body.eval(env, ctx);
                    env.pop_frame();
                    return result;
                }
            }
        }

        // No arm matched
        Err(EvalError::NonExhaustiveMatch {
            value: format!("{:?}", scrutinee),
            span: None,
        })
    }
}
```

---

## Loop Expressions

### src/eval/loops.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use crate::eval::control::ControlFlow;
use super::Evaluate;
use super::if_expr::eval_block;

// ═══════════════════════════════════════════════════════════════════════
// loop expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprLoop {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.name.ident.to_string());

        loop {
            // Check for interruption
            if ctx.is_interrupted() {
                return Err(EvalError::Interrupted);
            }

            // Evaluate body
            match eval_block(&self.body, env, ctx) {
                Ok(_) => {
                    // Continue looping
                }
                Err(EvalError::ControlFlow(cf)) => {
                    match &cf {
                        ControlFlow::Break { value, .. } if cf.matches_label(label.as_deref()) => {
                            return Ok(value.clone());
                        }
                        ControlFlow::Continue { .. } if cf.matches_label(label.as_deref()) => {
                            // Continue to next iteration
                        }
                        _ => {
                            // Propagate (different label or return)
                            return Err(EvalError::ControlFlow(cf));
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// while expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprWhile {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.name.ident.to_string());

        loop {
            // Check for interruption
            if ctx.is_interrupted() {
                return Err(EvalError::Interrupted);
            }

            // Evaluate condition
            let cond = self.cond.eval(env, ctx)?;
            let cond_bool = match cond {
                Value::Bool(b) => b,
                other => {
                    return Err(EvalError::TypeError {
                        message: format!(
                            "expected `bool` in while condition, found `{}`",
                            crate::error::type_name(&other)
                        ),
                        span: None,
                    });
                }
            };

            if !cond_bool {
                // Condition false, exit loop
                return Ok(Value::Unit);
            }

            // Evaluate body
            match eval_block(&self.body, env, ctx) {
                Ok(_) => {
                    // Continue looping
                }
                Err(EvalError::ControlFlow(cf)) => {
                    match &cf {
                        ControlFlow::Break { .. } if cf.matches_label(label.as_deref()) => {
                            // while loops always return unit (break value ignored)
                            return Ok(Value::Unit);
                        }
                        ControlFlow::Continue { .. } if cf.matches_label(label.as_deref()) => {
                            // Continue to next iteration
                        }
                        _ => {
                            // Propagate
                            return Err(EvalError::ControlFlow(cf));
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// for expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprForLoop {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.name.ident.to_string());

        // Evaluate the iterator expression
        let iter_value = self.expr.eval(env, ctx)?;

        // Convert to an iterator
        let iterator = value_to_iterator(iter_value)?;

        for item in iterator {
            // Check for interruption
            if ctx.is_interrupted() {
                return Err(EvalError::Interrupted);
            }

            // Push frame for loop body
            env.push_frame();

            // Bind the pattern
            if let Some(bindings) = super::pattern::match_pattern(&self.pat, &item, None)? {
                super::pattern::apply_bindings(env, bindings);
            } else {
                env.pop_frame();
                return Err(EvalError::RefutablePattern {
                    pattern: format!("{:?}", self.pat),
                    span: None,
                });
            }

            // Evaluate body
            let result = eval_block(&self.body, env, ctx);

            env.pop_frame();

            match result {
                Ok(_) => {
                    // Continue looping
                }
                Err(EvalError::ControlFlow(cf)) => {
                    match &cf {
                        ControlFlow::Break { .. } if cf.matches_label(label.as_deref()) => {
                            return Ok(Value::Unit);
                        }
                        ControlFlow::Continue { .. } if cf.matches_label(label.as_deref()) => {
                            // Continue to next iteration
                        }
                        _ => {
                            return Err(EvalError::ControlFlow(cf));
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(Value::Unit)
    }
}

/// Convert a Value to an iterator of Values.
fn value_to_iterator(value: Value) -> Result<Box<dyn Iterator<Item = Value>>, EvalError> {
    match value {
        Value::Vec(elements) => Ok(Box::new(elements.iter().cloned().collect::<Vec<_>>().into_iter())),
        Value::Array(elements) => Ok(Box::new(elements.iter().cloned().collect::<Vec<_>>().into_iter())),
        Value::String(s) => Ok(Box::new(
            s.chars().map(Value::Char).collect::<Vec<_>>().into_iter(),
        )),
        // Range values would go here if we had them
        other => Err(EvalError::TypeError {
            message: format!(
                "`{}` is not an iterator",
                crate::error::type_name(&other)
            ),
            span: None,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// break expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprBreak {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.ident.to_string());

        let value = if let Some(expr) = &self.expr {
            expr.eval(env, ctx)?
        } else {
            Value::Unit
        };

        Err(EvalError::ControlFlow(ControlFlow::Break { value, label }))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// continue expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprContinue {
    fn eval(&self, _env: &mut Environment, _ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.ident.to_string());

        Err(EvalError::ControlFlow(ControlFlow::Continue { label }))
    }
}
```

---

## Update Dispatcher

### Update src/eval/mod.rs

```rust
pub mod literal;
pub mod path;
pub mod unary;
pub mod binary;
pub mod control;
pub mod if_expr;
pub mod match_expr;
pub mod loops;
pub mod pattern;

// ... existing code ...

impl Evaluate for syn::Expr {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        if ctx.is_interrupted() {
            return Err(EvalError::Interrupted);
        }

        match self {
            // Stage 1.3: Basic expressions
            syn::Expr::Lit(expr) => expr.eval(env, ctx),
            syn::Expr::Path(expr) => expr.eval(env, ctx),
            syn::Expr::Unary(expr) => expr.eval(env, ctx),
            syn::Expr::Binary(expr) => expr.eval(env, ctx),

            // Stage 1.4: Control flow
            syn::Expr::If(expr) => expr.eval(env, ctx),
            syn::Expr::Match(expr) => expr.eval(env, ctx),
            syn::Expr::Loop(expr) => expr.eval(env, ctx),
            syn::Expr::While(expr) => expr.eval(env, ctx),
            syn::Expr::ForLoop(expr) => expr.eval(env, ctx),
            syn::Expr::Break(expr) => expr.eval(env, ctx),
            syn::Expr::Continue(expr) => expr.eval(env, ctx),

            // Stage 1.5: Functions (not yet implemented)
            syn::Expr::Call(_) => Err(not_yet_implemented("function call", self)),
            syn::Expr::MethodCall(_) => Err(not_yet_implemented("method call", self)),
            syn::Expr::Closure(_) => Err(not_yet_implemented("closure", self)),
            syn::Expr::Return(_) => Err(not_yet_implemented("return", self)),

            // Stage 1.6: Blocks
            syn::Expr::Block(expr) => if_expr::eval_block(&expr.block, env, ctx),

            // Parenthesized expressions - just unwrap
            syn::Expr::Paren(expr) => expr.expr.eval(env, ctx),
            syn::Expr::Group(expr) => expr.expr.eval(env, ctx),

            // Everything else
            _ => Err(EvalError::UnsupportedExpr {
                kind: expr_kind_name(self).to_string(),
                span: Some(expr_span(self)),
            }),
        }
    }
}

// Re-export for use by other modules
pub use if_expr::eval_block;
pub use pattern::{match_pattern, apply_bindings};
```

---

## Update lib.rs Exports

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
pub use eval::{Evaluate, eval_expr, eval_block, ControlFlow};
```

---

## Test Cases

### tests/control_flow_tests.rs

```rust
use treebeard_core::*;

fn eval(src: &str) -> Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    expr.eval(&mut env, &ctx)
}

fn eval_with_env(src: &str, env: &mut Environment) -> Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let ctx = EvalContext::default();
    expr.eval(env, &ctx)
}

// ═══════════════════════════════════════════════════════════════════════
// If Expressions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_if_true_branch() {
    assert_eq!(eval("if true { 1 } else { 2 }").unwrap(), Value::I64(1));
}

#[test]
fn test_if_false_branch() {
    assert_eq!(eval("if false { 1 } else { 2 }").unwrap(), Value::I64(2));
}

#[test]
fn test_if_no_else() {
    assert_eq!(eval("if false { 1 }").unwrap(), Value::Unit);
    assert_eq!(eval("if true { 1 }").unwrap(), Value::I64(1));
}

#[test]
fn test_if_else_if() {
    let code = r#"
        if false { 1 }
        else if false { 2 }
        else if true { 3 }
        else { 4 }
    "#;
    assert_eq!(eval(code).unwrap(), Value::I64(3));
}

#[test]
fn test_if_with_condition() {
    let mut env = Environment::new();
    env.define("x", Value::I64(10));
    assert_eq!(
        eval_with_env("if x > 5 { 1 } else { 2 }", &mut env).unwrap(),
        Value::I64(1)
    );
}

#[test]
fn test_if_non_bool_condition() {
    let result = eval("if 42 { 1 } else { 2 }");
    assert!(matches!(result, Err(EvalError::TypeError { .. })));
}

#[test]
fn test_if_nested() {
    let code = r#"
        if true {
            if false { 1 } else { 2 }
        } else {
            3
        }
    "#;
    assert_eq!(eval(code).unwrap(), Value::I64(2));
}

// ═══════════════════════════════════════════════════════════════════════
// Match Expressions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_match_literal() {
    let mut env = Environment::new();
    env.define("x", Value::I64(2));

    let code = r#"
        match x {
            1 => 10,
            2 => 20,
            3 => 30,
            _ => 0,
        }
    "#;
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(20));
}

#[test]
fn test_match_wildcard() {
    let mut env = Environment::new();
    env.define("x", Value::I64(999));

    let code = "match x { _ => 42 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(42));
}

#[test]
fn test_match_binding() {
    let mut env = Environment::new();
    env.define("x", Value::I64(5));

    let code = "match x { n => n + 1 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(6));
}

#[test]
fn test_match_or_pattern() {
    let mut env = Environment::new();
    env.define("x", Value::I64(2));

    let code = "match x { 1 | 2 | 3 => 100, _ => 0 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(100));
}

#[test]
fn test_match_guard() {
    let mut env = Environment::new();
    env.define("x", Value::I64(10));

    let code = r#"
        match x {
            n if n > 5 => 1,
            n if n > 0 => 2,
            _ => 3,
        }
    "#;
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(1));
}

#[test]
fn test_match_tuple() {
    let mut env = Environment::new();
    env.define("pair", Value::Tuple(vec![Value::I64(1), Value::I64(2)].into()));

    let code = "match pair { (a, b) => a + b }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(3));
}

#[test]
fn test_match_option_some() {
    let mut env = Environment::new();
    env.define("opt", Value::Option(Some(Box::new(Value::I64(42)))));

    let code = "match opt { Some(x) => x, None => 0 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(42));
}

#[test]
fn test_match_option_none() {
    let mut env = Environment::new();
    env.define("opt", Value::Option(None));

    let code = "match opt { Some(x) => x, None => 0 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(0));
}

#[test]
fn test_match_non_exhaustive() {
    let mut env = Environment::new();
    env.define("x", Value::I64(5));

    let code = "match x { 1 => 10, 2 => 20 }";
    let result = eval_with_env(code, &mut env);
    assert!(matches!(result, Err(EvalError::NonExhaustiveMatch { .. })));
}

#[test]
fn test_match_range_inclusive() {
    let mut env = Environment::new();
    env.define("x", Value::I64(5));

    let code = "match x { 1..=10 => true, _ => false }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Loop Expressions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_loop_break_value() {
    let code = "loop { break 42 }";
    assert_eq!(eval(code).unwrap(), Value::I64(42));
}

#[test]
fn test_loop_break_unit() {
    let code = "loop { break }";
    assert_eq!(eval(code).unwrap(), Value::Unit);
}

#[test]
fn test_loop_with_counter() {
    let mut env = Environment::new();
    env.define_with_mode("i", Value::I64(0), BindingMode::Mutable);

    // This requires assignment which we don't have yet, so we'll test a simpler case
    let code = "loop { break 100 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(100));
}

#[test]
fn test_loop_labeled_break() {
    let code = r#"
        'outer: loop {
            loop {
                break 'outer 42
            }
        }
    "#;
    assert_eq!(eval(code).unwrap(), Value::I64(42));
}

// ═══════════════════════════════════════════════════════════════════════
// While Expressions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_while_false_condition() {
    let code = "while false { 1 }";
    assert_eq!(eval(code).unwrap(), Value::Unit);
}

#[test]
fn test_while_immediate_break() {
    let code = "while true { break }";
    assert_eq!(eval(code).unwrap(), Value::Unit);
}

#[test]
fn test_while_non_bool_condition() {
    let result = eval("while 42 { break }");
    assert!(matches!(result, Err(EvalError::TypeError { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// For Expressions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_for_over_vec() {
    let mut env = Environment::new();
    env.define("items", Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]));
    env.define_with_mode("sum", Value::I64(0), BindingMode::Mutable);

    // Without assignment, we just test iteration works
    let code = "for _x in items { }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::Unit);
}

#[test]
fn test_for_over_string() {
    let mut env = Environment::new();
    env.define("s", Value::string("abc"));

    let code = "for _c in s { }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::Unit);
}

#[test]
fn test_for_break() {
    let mut env = Environment::new();
    env.define("items", Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]));

    let code = "for _x in items { break }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::Unit);
}

#[test]
fn test_for_continue() {
    let mut env = Environment::new();
    env.define("items", Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]));

    let code = "for _x in items { continue }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::Unit);
}

#[test]
fn test_for_with_pattern() {
    let mut env = Environment::new();
    env.define(
        "pairs",
        Value::vec(vec![
            Value::Tuple(vec![Value::I64(1), Value::I64(2)].into()),
            Value::Tuple(vec![Value::I64(3), Value::I64(4)].into()),
        ]),
    );

    let code = "for (_a, _b) in pairs { }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::Unit);
}

// ═══════════════════════════════════════════════════════════════════════
// Break and Continue
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_break_outside_loop() {
    // break at top level should propagate as ControlFlow error
    let result = eval("break");
    assert!(matches!(result, Err(EvalError::ControlFlow(_))));
}

#[test]
fn test_continue_outside_loop() {
    let result = eval("continue");
    assert!(matches!(result, Err(EvalError::ControlFlow(_))));
}

#[test]
fn test_labeled_continue() {
    let code = r#"
        'outer: loop {
            loop {
                break 'outer 42
            }
        }
    "#;
    assert_eq!(eval(code).unwrap(), Value::I64(42));
}

// ═══════════════════════════════════════════════════════════════════════
// Block Expressions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_block_expression() {
    let code = "{ 1; 2; 3 }";
    assert_eq!(eval(code).unwrap(), Value::I64(3));
}

#[test]
fn test_block_with_semicolon() {
    let code = "{ 1; 2; 3; }";
    assert_eq!(eval(code).unwrap(), Value::Unit);
}

#[test]
fn test_block_empty() {
    let code = "{ }";
    assert_eq!(eval(code).unwrap(), Value::Unit);
}

#[test]
fn test_block_nested() {
    let code = "{ { { 42 } } }";
    assert_eq!(eval(code).unwrap(), Value::I64(42));
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern Matching Details
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_pattern_at() {
    let mut env = Environment::new();
    env.define("x", Value::I64(5));

    let code = "match x { n @ 1..=10 => n, _ => 0 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(5));
}

#[test]
fn test_pattern_slice() {
    let mut env = Environment::new();
    env.define("arr", Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]));

    let code = "match arr { [a, b, c] => a + b + c, _ => 0 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(6));
}

#[test]
fn test_pattern_slice_rest() {
    let mut env = Environment::new();
    env.define("arr", Value::vec(vec![
        Value::I64(1), Value::I64(2), Value::I64(3), Value::I64(4)
    ]));

    let code = "match arr { [first, .., last] => first + last, _ => 0 }";
    assert_eq!(eval_with_env(code, &mut env).unwrap(), Value::I64(5));
}

// ═══════════════════════════════════════════════════════════════════════
// Complex Control Flow
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_if_in_loop() {
    let code = r#"
        loop {
            if true {
                break 42
            }
        }
    "#;
    assert_eq!(eval(code).unwrap(), Value::I64(42));
}

#[test]
fn test_match_in_loop() {
    let code = r#"
        loop {
            match true {
                true => break 100,
                false => break 0,
            }
        }
    "#;
    assert_eq!(eval(code).unwrap(), Value::I64(100));
}

#[test]
fn test_nested_loops_break_inner() {
    let code = r#"
        loop {
            loop {
                break
            };
            break 42
        }
    "#;
    assert_eq!(eval(code).unwrap(), Value::I64(42));
}
```

---

## Completion Checklist

- [ ] Create `src/eval/control.rs` with `ControlFlow` enum
- [ ] Add `ControlFlow` error variant to `EvalError`
- [ ] Create `src/eval/if_expr.rs` with `ExprIf` evaluation
- [ ] Implement `eval_block` helper function
- [ ] Create `src/eval/pattern.rs` with pattern matching
- [ ] Implement patterns: wildcard, ident, literal, or, tuple, struct, tuple-struct, path, range, slice
- [ ] Create `src/eval/match_expr.rs` with `ExprMatch` evaluation
- [ ] Implement match guards
- [ ] Create `src/eval/loops.rs` with loop expressions
- [ ] Implement `loop` with break value
- [ ] Implement `while` loop
- [ ] Implement `for` loop with iterator conversion
- [ ] Implement `break` with optional value and label
- [ ] Implement `continue` with optional label
- [ ] Handle labeled loops (`'label: loop { }`)
- [ ] Update `src/eval/mod.rs` dispatcher
- [ ] Update `lib.rs` exports
- [ ] All tests passing

---

## Design Notes

### Why Use Error Path for Control Flow?

`break` and `continue` are non-local jumps - they exit the current expression and propagate up to the enclosing loop. Using `Err(EvalError::ControlFlow(...))` lets us reuse Rust's `?` operator for propagation while still distinguishing from real errors.

### Why ControlFlow::Break Has a Value?

In Rust, `loop { break 42 }` evaluates to `42`. The `break` expression carries the loop's return value. `while` and `for` always return `()`, so they ignore any break value.

### Why Pattern Matching is Complex?

Rust has rich patterns: literals, bindings, wildcards, structs, tuples, slices with rest, ranges, guards, and more. We implement the most common ones; exotic patterns can be added later.

### Why Check Labels?

Labeled loops (`'outer: loop { }`) allow breaking/continuing specific loops from nested positions. The label matching ensures `break 'outer` only exits the loop labeled `'outer`.

---

## Next Stage

**Stage 1.5: Functions** — Implement `fn` definitions, function calls, argument passing, and `return` statements.
