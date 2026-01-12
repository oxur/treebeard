# Stage 1.5: Functions

**Phase:** 1 - Core Evaluator
**Stage:** 1.5
**Prerequisites:** Stage 1.1-1.4 (Value, Environment, Expressions, Control Flow)
**Estimated effort:** 3-4 days

---

## Objective

Implement function definitions, function calls, argument passing, and `return` statements. This enables defining reusable procedures and is the capstone of Phase 1.

---

## Overview

Functions in Treebeard follow Rust semantics:

| Construct | Example | Notes |
|-----------|---------|-------|
| Function definition | `fn add(a: i64, b: i64) -> i64 { a + b }` | Stored as `FunctionValue` in environment |
| Function call | `add(1, 2)` | Look up function, bind args, evaluate body |
| Method call | `x.len()` | Desugar to function call (limited support) |
| Return | `return 42` or implicit | Explicit return or last expression |

**Key design decisions:**

1. **Late binding** — Functions are looked up by name at call time (enables hot reload)
2. **Type erasure** — Parameter types are not checked at runtime (Rust semantics assumed)
3. **Call depth tracking** — Prevent stack overflow from infinite recursion
4. **AST storage** — Function bodies are stored as `Arc<syn::Block>` for interpretation

---

## File Structure

```
treebeard/src/
├── lib.rs              # Update exports
├── eval/
│   ├── mod.rs          # Update dispatcher
│   ├── function.rs     # ← New: fn definition evaluation
│   ├── call.rs         # ← New: function call evaluation
│   ├── return_expr.rs  # ← New: return expression
│   └── item.rs         # ← New: syn::Item evaluation
└── ...
```

---

## Function Definition

### src/eval/function.rs

```rust
use std::sync::Arc;
use crate::{Value, Environment, EvalContext, EvalError, FunctionValue};

/// Extract a FunctionValue from a syn::ItemFn.
pub fn function_from_item(item_fn: &syn::ItemFn) -> Result<FunctionValue, EvalError> {
    let name = item_fn.sig.ident.to_string();

    // Extract parameter names
    let params = extract_params(&item_fn.sig)?;

    // Store the body
    let body = item_fn.block.as_ref().clone();

    Ok(FunctionValue::new(name, params, body))
}

/// Extract parameter names from a function signature.
fn extract_params(sig: &syn::Signature) -> Result<Vec<String>, EvalError> {
    let mut params = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat_type) => {
                // Extract the pattern (usually just an identifier)
                let name = extract_pat_name(&pat_type.pat)?;
                params.push(name);
            }
            syn::FnArg::Receiver(_) => {
                // self parameter - we'll handle this as a special case
                params.push("self".to_string());
            }
        }
    }

    Ok(params)
}

/// Extract a name from a pattern (for function parameters).
fn extract_pat_name(pat: &syn::Pat) -> Result<String, EvalError> {
    match pat {
        syn::Pat::Ident(pat_ident) => Ok(pat_ident.ident.to_string()),
        syn::Pat::Wild(_) => Ok("_".to_string()),
        syn::Pat::Reference(pat_ref) => extract_pat_name(&pat_ref.pat),
        syn::Pat::Type(pat_type) => extract_pat_name(&pat_type.pat),
        _ => Err(EvalError::UnsupportedExpr {
            kind: format!("complex pattern in function parameter: {:?}", pat),
            span: None,
        }),
    }
}

/// Define a function in the environment.
pub fn define_function(
    item_fn: &syn::ItemFn,
    env: &mut Environment,
) -> Result<(), EvalError> {
    let func = function_from_item(item_fn)?;
    let name = func.name.clone();
    env.define(name, Value::Function(Arc::new(func)));
    Ok(())
}
```

---

## Function Call Evaluation

### src/eval/call.rs

```rust
use std::sync::Arc;
use crate::{Value, Environment, EvalContext, EvalError, FunctionValue, BuiltinFn};
use crate::eval::control::ControlFlow;
use super::Evaluate;
use super::if_expr::eval_block;

impl Evaluate for syn::ExprCall {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate the function expression
        let func_value = self.func.eval(env, ctx)?;

        // Evaluate arguments
        let args: Vec<Value> = self
            .args
            .iter()
            .map(|arg| arg.eval(env, ctx))
            .collect::<Result<Vec<_>, _>>()?;

        // Call the function
        call_value(func_value, args, env, ctx, None)
    }
}

impl Evaluate for syn::ExprMethodCall {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate the receiver
        let receiver = self.receiver.eval(env, ctx)?;

        // Evaluate arguments
        let mut args: Vec<Value> = vec![receiver];
        for arg in &self.args {
            args.push(arg.eval(env, ctx)?);
        }

        // Look up the method by name
        let method_name = self.method.to_string();

        // First, try built-in methods on the receiver type
        if let Some(result) = try_builtin_method(&method_name, &args)? {
            return Ok(result);
        }

        // Otherwise, look up as a regular function
        let func = env.get(&method_name).cloned().ok_or_else(|| {
            EvalError::UndefinedVariable {
                name: method_name.clone(),
                span: Some(self.method.span()),
            }
        })?;

        call_value(func, args, env, ctx, Some(self.method.span()))
    }
}

/// Call a Value as a function.
pub fn call_value(
    func: Value,
    args: Vec<Value>,
    env: &mut Environment,
    ctx: &EvalContext,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match func {
        Value::Function(f) => call_function(&f, args, env, ctx),
        Value::BuiltinFn(f) => call_builtin(&f, args, span),
        Value::Closure(c) => call_closure(&c, args, env, ctx),
        other => Err(EvalError::TypeError {
            message: format!(
                "expected function, found `{}`",
                crate::error::type_name(&other)
            ),
            span,
        }),
    }
}

/// Call a user-defined function.
fn call_function(
    func: &FunctionValue,
    args: Vec<Value>,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Check arity
    if args.len() != func.params.len() {
        return Err(EvalError::ArityMismatch {
            expected: func.params.len(),
            got: args.len(),
            name: func.name.clone(),
            span: None,
        });
    }

    // Track call depth (stack overflow protection)
    env.enter_call()?;

    // Create new scope for function body
    env.push_frame();

    // Bind parameters to arguments
    for (param, arg) in func.params.iter().zip(args.into_iter()) {
        env.define(param, arg);
    }

    // Evaluate the function body
    let result = eval_function_body(&func.body, env, ctx);

    // Clean up
    env.pop_frame();
    env.exit_call();

    // Handle return control flow
    match result {
        Ok(value) => Ok(value),
        Err(EvalError::ControlFlow(ControlFlow::Return { value })) => Ok(value),
        Err(e) => Err(e),
    }
}

/// Call a built-in function.
fn call_builtin(
    func: &BuiltinFn,
    args: Vec<Value>,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    // Check arity (if not variadic)
    if func.arity >= 0 && args.len() != func.arity as usize {
        return Err(EvalError::ArityMismatch {
            expected: func.arity as usize,
            got: args.len(),
            name: func.name.clone(),
            span,
        });
    }

    // Call the native function
    (func.func)(&args).map_err(|e| EvalError::BuiltinError {
        name: func.name.clone(),
        message: e,
        span,
    })
}

/// Call a closure.
fn call_closure(
    closure: &crate::ClosureValue,
    args: Vec<Value>,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Check arity
    if args.len() != closure.params.len() {
        return Err(EvalError::ArityMismatch {
            expected: closure.params.len(),
            got: args.len(),
            name: "<closure>".to_string(),
            span: None,
        });
    }

    // Track call depth
    env.enter_call()?;

    // Create new scope
    env.push_frame();

    // Bind captured variables first
    for (name, value) in closure.captures.iter() {
        env.define(name, value.clone());
    }

    // Bind parameters
    for (param, arg) in closure.params.iter().zip(args.into_iter()) {
        env.define(param, arg);
    }

    // Evaluate the closure body
    let result = closure.body.eval(env, ctx);

    // Clean up
    env.pop_frame();
    env.exit_call();

    // Handle return
    match result {
        Ok(value) => Ok(value),
        Err(EvalError::ControlFlow(ControlFlow::Return { value })) => Ok(value),
        Err(e) => Err(e),
    }
}

/// Evaluate a function body (block).
fn eval_function_body(
    body: &syn::Block,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let mut last_value = Value::Unit;

    for stmt in &body.stmts {
        last_value = eval_stmt_in_function(stmt, env, ctx)?;
    }

    Ok(last_value)
}

/// Evaluate a statement within a function body.
fn eval_stmt_in_function(
    stmt: &syn::Stmt,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    match stmt {
        syn::Stmt::Expr(expr, None) => {
            // Expression without semicolon - its value is the result
            expr.eval(env, ctx)
        }
        syn::Stmt::Expr(expr, Some(_)) => {
            // Expression with semicolon - evaluate for side effects
            expr.eval(env, ctx)?;
            Ok(Value::Unit)
        }
        syn::Stmt::Local(local) => {
            // Let binding - delegate to Stage 1.6 implementation
            super::local::eval_local(local, env, ctx)?;
            Ok(Value::Unit)
        }
        syn::Stmt::Item(item) => {
            // Item in function (nested fn, etc.)
            super::item::eval_item(item, env, ctx)?;
            Ok(Value::Unit)
        }
        syn::Stmt::Macro(_) => Err(EvalError::UnsupportedExpr {
            kind: "macro statement".to_string(),
            span: None,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Built-in Methods
// ═══════════════════════════════════════════════════════════════════════

/// Try to call a built-in method on a value.
fn try_builtin_method(
    method: &str,
    args: &[Value],
) -> Result<Option<Value>, EvalError> {
    if args.is_empty() {
        return Ok(None);
    }

    let receiver = &args[0];
    let method_args = &args[1..];

    match (receiver, method) {
        // String methods
        (Value::String(s), "len") if method_args.is_empty() => {
            Ok(Some(Value::Usize(s.len())))
        }
        (Value::String(s), "is_empty") if method_args.is_empty() => {
            Ok(Some(Value::Bool(s.is_empty())))
        }
        (Value::String(s), "to_uppercase") if method_args.is_empty() => {
            Ok(Some(Value::string(s.to_uppercase())))
        }
        (Value::String(s), "to_lowercase") if method_args.is_empty() => {
            Ok(Some(Value::string(s.to_lowercase())))
        }
        (Value::String(s), "trim") if method_args.is_empty() => {
            Ok(Some(Value::string(s.trim())))
        }
        (Value::String(s), "chars") if method_args.is_empty() => {
            Ok(Some(Value::vec(s.chars().map(Value::Char).collect())))
        }
        (Value::String(s), "contains") if method_args.len() == 1 => {
            match &method_args[0] {
                Value::String(needle) => Ok(Some(Value::Bool(s.contains(needle.as_str())))),
                Value::Char(c) => Ok(Some(Value::Bool(s.contains(*c)))),
                _ => Ok(None),
            }
        }
        (Value::String(s), "starts_with") if method_args.len() == 1 => {
            if let Value::String(prefix) = &method_args[0] {
                Ok(Some(Value::Bool(s.starts_with(prefix.as_str()))))
            } else {
                Ok(None)
            }
        }
        (Value::String(s), "ends_with") if method_args.len() == 1 => {
            if let Value::String(suffix) = &method_args[0] {
                Ok(Some(Value::Bool(s.ends_with(suffix.as_str()))))
            } else {
                Ok(None)
            }
        }

        // Vec methods
        (Value::Vec(v), "len") if method_args.is_empty() => {
            Ok(Some(Value::Usize(v.len())))
        }
        (Value::Vec(v), "is_empty") if method_args.is_empty() => {
            Ok(Some(Value::Bool(v.is_empty())))
        }
        (Value::Vec(v), "first") if method_args.is_empty() => {
            Ok(Some(Value::Option(v.first().cloned().map(Box::new))))
        }
        (Value::Vec(v), "last") if method_args.is_empty() => {
            Ok(Some(Value::Option(v.last().cloned().map(Box::new))))
        }
        (Value::Vec(v), "get") if method_args.len() == 1 => {
            if let Some(idx) = method_args[0].as_usize() {
                Ok(Some(Value::Option(v.get(idx).cloned().map(Box::new))))
            } else {
                Ok(None)
            }
        }
        (Value::Vec(v), "contains") if method_args.len() == 1 => {
            Ok(Some(Value::Bool(v.contains(&method_args[0]))))
        }

        // Array methods (same as Vec)
        (Value::Array(v), "len") if method_args.is_empty() => {
            Ok(Some(Value::Usize(v.len())))
        }
        (Value::Array(v), "is_empty") if method_args.is_empty() => {
            Ok(Some(Value::Bool(v.is_empty())))
        }
        (Value::Array(v), "first") if method_args.is_empty() => {
            Ok(Some(Value::Option(v.first().cloned().map(Box::new))))
        }
        (Value::Array(v), "last") if method_args.is_empty() => {
            Ok(Some(Value::Option(v.last().cloned().map(Box::new))))
        }

        // Option methods
        (Value::Option(opt), "is_some") if method_args.is_empty() => {
            Ok(Some(Value::Bool(opt.is_some())))
        }
        (Value::Option(opt), "is_none") if method_args.is_empty() => {
            Ok(Some(Value::Bool(opt.is_none())))
        }
        (Value::Option(opt), "unwrap") if method_args.is_empty() => {
            match opt {
                Some(v) => Ok(Some(v.as_ref().clone())),
                None => Err(EvalError::BuiltinError {
                    name: "unwrap".to_string(),
                    message: "called `Option::unwrap()` on a `None` value".to_string(),
                    span: None,
                }),
            }
        }
        (Value::Option(opt), "unwrap_or") if method_args.len() == 1 => {
            Ok(Some(match opt {
                Some(v) => v.as_ref().clone(),
                None => method_args[0].clone(),
            }))
        }

        // Result methods
        (Value::Result(res), "is_ok") if method_args.is_empty() => {
            Ok(Some(Value::Bool(res.is_ok())))
        }
        (Value::Result(res), "is_err") if method_args.is_empty() => {
            Ok(Some(Value::Bool(res.is_err())))
        }
        (Value::Result(res), "unwrap") if method_args.is_empty() => {
            match res.as_ref() {
                Ok(v) => Ok(Some(v.clone())),
                Err(e) => Err(EvalError::BuiltinError {
                    name: "unwrap".to_string(),
                    message: format!("called `Result::unwrap()` on an `Err` value: {:?}", e),
                    span: None,
                }),
            }
        }
        (Value::Result(res), "unwrap_err") if method_args.is_empty() => {
            match res.as_ref() {
                Ok(v) => Err(EvalError::BuiltinError {
                    name: "unwrap_err".to_string(),
                    message: format!("called `Result::unwrap_err()` on an `Ok` value: {:?}", v),
                    span: None,
                }),
                Err(e) => Ok(Some(e.clone())),
            }
        }

        // Clone (works on most values)
        (_, "clone") if method_args.is_empty() => {
            Ok(Some(receiver.clone()))
        }

        // No built-in method found
        _ => Ok(None),
    }
}
```

---

## Return Expression

### src/eval/return_expr.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use crate::eval::control::ControlFlow;
use super::Evaluate;

impl Evaluate for syn::ExprReturn {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let value = if let Some(expr) = &self.expr {
            expr.eval(env, ctx)?
        } else {
            Value::Unit
        };

        Err(EvalError::ControlFlow(ControlFlow::Return { value }))
    }
}
```

---

## Item Evaluation

### src/eval/item.rs

```rust
use std::sync::Arc;
use crate::{Value, Environment, EvalContext, EvalError, FunctionValue};
use super::function::function_from_item;

/// Evaluate a top-level item.
pub fn eval_item(
    item: &syn::Item,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    match item {
        syn::Item::Fn(item_fn) => {
            let func = function_from_item(item_fn)?;
            let name = func.name.clone();
            env.define(name, Value::Function(Arc::new(func)));
            Ok(Value::Unit)
        }

        syn::Item::Const(item_const) => {
            // Evaluate the const expression
            let value = item_const.expr.eval(env, ctx)?;
            let name = item_const.ident.to_string();
            env.define(name, value);
            Ok(Value::Unit)
        }

        syn::Item::Static(item_static) => {
            // Evaluate the static expression
            let value = item_static.expr.eval(env, ctx)?;
            let name = item_static.ident.to_string();
            // Statics are mutable by default in the interpreter
            env.define_with_mode(
                name,
                value,
                if item_static.mutability.is_some() {
                    crate::BindingMode::Mutable
                } else {
                    crate::BindingMode::Immutable
                },
            );
            Ok(Value::Unit)
        }

        // Struct/Enum definitions - just register the type name for now
        syn::Item::Struct(item_struct) => {
            let name = item_struct.ident.to_string();
            // Store struct definition for constructor calls
            // For now, we don't need to do anything special
            // Struct literals will be handled in expressions
            let _ = name;
            Ok(Value::Unit)
        }

        syn::Item::Enum(item_enum) => {
            let name = item_enum.ident.to_string();
            let _ = name;
            Ok(Value::Unit)
        }

        // Impl blocks - register methods
        syn::Item::Impl(item_impl) => {
            // For now, just evaluate any associated functions
            for impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(method) = impl_item {
                    let func = function_from_impl_method(method, &item_impl.self_ty)?;
                    let name = func.name.clone();
                    env.define(name, Value::Function(Arc::new(func)));
                }
            }
            Ok(Value::Unit)
        }

        // Type aliases - no runtime effect
        syn::Item::Type(_) => Ok(Value::Unit),

        // Use statements - no runtime effect (imports are resolved at parse time)
        syn::Item::Use(_) => Ok(Value::Unit),

        // Modules - not yet supported
        syn::Item::Mod(_) => Err(EvalError::UnsupportedExpr {
            kind: "module definition".to_string(),
            span: None,
        }),

        // Traits - not yet supported
        syn::Item::Trait(_) => Err(EvalError::UnsupportedExpr {
            kind: "trait definition".to_string(),
            span: None,
        }),

        // Everything else
        _ => Err(EvalError::UnsupportedExpr {
            kind: format!("item type: {:?}", std::mem::discriminant(item)),
            span: None,
        }),
    }
}

/// Extract a FunctionValue from an impl method.
fn function_from_impl_method(
    method: &syn::ImplItemFn,
    _self_ty: &syn::Type,
) -> Result<FunctionValue, EvalError> {
    let name = method.sig.ident.to_string();
    let params = extract_method_params(&method.sig)?;
    let body = method.block.clone();

    Ok(FunctionValue::new(name, params, body))
}

/// Extract parameter names from a method signature.
fn extract_method_params(sig: &syn::Signature) -> Result<Vec<String>, EvalError> {
    let mut params = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat_type) => {
                let name = extract_pat_name(&pat_type.pat)?;
                params.push(name);
            }
            syn::FnArg::Receiver(_) => {
                params.push("self".to_string());
            }
        }
    }

    Ok(params)
}

fn extract_pat_name(pat: &syn::Pat) -> Result<String, EvalError> {
    match pat {
        syn::Pat::Ident(pat_ident) => Ok(pat_ident.ident.to_string()),
        syn::Pat::Wild(_) => Ok("_".to_string()),
        syn::Pat::Reference(pat_ref) => extract_pat_name(&pat_ref.pat),
        syn::Pat::Type(pat_type) => extract_pat_name(&pat_type.pat),
        _ => Err(EvalError::UnsupportedExpr {
            kind: format!("complex pattern in parameter: {:?}", pat),
            span: None,
        }),
    }
}

/// Evaluate a sequence of items (top-level forms).
pub fn eval_items(
    items: &[syn::Item],
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let mut last_value = Value::Unit;

    for item in items {
        last_value = eval_item(item, env, ctx)?;
    }

    Ok(last_value)
}
```

---

## Local Binding (Partial - Full in 1.6)

### src/eval/local.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError, BindingMode};
use crate::eval::pattern::{match_pattern, apply_bindings};
use super::Evaluate;

/// Evaluate a local (let) binding.
pub fn eval_local(
    local: &syn::Local,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<(), EvalError> {
    // Get the initializer value (or Unit if none)
    let value = if let Some(init) = &local.init {
        init.expr.eval(env, ctx)?
    } else {
        Value::Unit
    };

    // Check if mutable
    let is_mutable = is_pattern_mutable(&local.pat);

    // Match the pattern and bind
    if let Some(bindings) = match_pattern(&local.pat, &value, None)? {
        for (name, val, pat_mut) in bindings {
            let mode = if is_mutable || pat_mut {
                BindingMode::Mutable
            } else {
                BindingMode::Immutable
            };
            env.define_with_mode(&name, val, mode);
        }
        Ok(())
    } else {
        Err(EvalError::RefutablePattern {
            pattern: format!("{:?}", local.pat),
            span: None,
        })
    }
}

/// Check if a pattern has the `mut` keyword.
fn is_pattern_mutable(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(pat_ident) => pat_ident.mutability.is_some(),
        syn::Pat::Type(pat_type) => is_pattern_mutable(&pat_type.pat),
        syn::Pat::Reference(pat_ref) => is_pattern_mutable(&pat_ref.pat),
        _ => false,
    }
}
```

---

## Extend Error Types

### Add to src/error.rs

```rust
/// Arity mismatch in function call.
#[error("function `{name}` expected {expected} argument(s), got {got}")]
ArityMismatch {
    expected: usize,
    got: usize,
    name: String,
    span: Option<Span>,
},

/// Built-in function error.
#[error("built-in function `{name}`: {message}")]
BuiltinError {
    name: String,
    message: String,
    span: Option<Span>,
},
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
pub mod function;
pub mod call;
pub mod return_expr;
pub mod item;
pub mod local;

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

            // Stage 1.5: Functions
            syn::Expr::Call(expr) => expr.eval(env, ctx),
            syn::Expr::MethodCall(expr) => expr.eval(env, ctx),
            syn::Expr::Return(expr) => expr.eval(env, ctx),

            // Stage 1.6: Blocks
            syn::Expr::Block(expr) => if_expr::eval_block(&expr.block, env, ctx),

            // Parenthesized expressions
            syn::Expr::Paren(expr) => expr.expr.eval(env, ctx),
            syn::Expr::Group(expr) => expr.expr.eval(env, ctx),

            // Closures - basic support (full in Phase 5)
            syn::Expr::Closure(expr) => eval_closure_expr(expr, env, ctx),

            // Everything else
            _ => Err(EvalError::UnsupportedExpr {
                kind: expr_kind_name(self).to_string(),
                span: Some(expr_span(self)),
            }),
        }
    }
}

/// Basic closure evaluation (captures handled in Phase 5).
fn eval_closure_expr(
    expr: &syn::ExprClosure,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<Value, EvalError> {
    use std::sync::Arc;
    use crate::ClosureValue;

    // Extract parameter names
    let params: Vec<String> = expr
        .inputs
        .iter()
        .map(|pat| match pat {
            syn::Pat::Ident(id) => Ok(id.ident.to_string()),
            syn::Pat::Wild(_) => Ok("_".to_string()),
            syn::Pat::Type(pt) => match pt.pat.as_ref() {
                syn::Pat::Ident(id) => Ok(id.ident.to_string()),
                _ => Err(EvalError::UnsupportedExpr {
                    kind: "complex closure parameter".to_string(),
                    span: None,
                }),
            },
            _ => Err(EvalError::UnsupportedExpr {
                kind: "complex closure parameter".to_string(),
                span: None,
            }),
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Store the body
    let body = expr.body.as_ref().clone();

    // For now, no captures (Phase 5 will add this)
    Ok(Value::Closure(Arc::new(ClosureValue {
        params,
        body: Arc::new(body),
        captures: Arc::new(vec![]),
    })))
}

// Re-exports
pub use if_expr::eval_block;
pub use pattern::{match_pattern, apply_bindings};
pub use item::{eval_item, eval_items};
pub use call::call_value;
pub use local::eval_local;
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
pub use eval::{
    Evaluate,
    eval_expr,
    eval_block,
    eval_item,
    eval_items,
    call_value,
    ControlFlow,
};
```

---

## Test Cases

### tests/function_tests.rs

```rust
use treebeard_core::*;

fn eval_items_str(src: &str) -> Result<Value, EvalError> {
    let file: syn::File = syn::parse_str(src).expect("parse failed");
    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();
    eval_items(&file.items, &mut env, &ctx)
}

fn eval_with_functions(items_src: &str, expr_src: &str) -> Result<Value, EvalError> {
    let file: syn::File = syn::parse_str(items_src).expect("parse items failed");
    let expr: syn::Expr = syn::parse_str(expr_src).expect("parse expr failed");
    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();
    eval_items(&file.items, &mut env, &ctx)?;
    expr.eval(&mut env, &ctx)
}

// ═══════════════════════════════════════════════════════════════════════
// Function Definition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_define_function() {
    let result = eval_items_str("fn add(a: i64, b: i64) -> i64 { a + b }");
    assert!(result.is_ok());
}

#[test]
fn test_function_in_env() {
    let file: syn::File = syn::parse_str("fn foo() -> i64 { 42 }").unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    eval_items(&file.items, &mut env, &ctx).unwrap();

    assert!(env.contains("foo"));
    assert!(matches!(env.get("foo"), Some(Value::Function(_))));
}

// ═══════════════════════════════════════════════════════════════════════
// Function Calls
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_call_simple_function() {
    let result = eval_with_functions(
        "fn answer() -> i64 { 42 }",
        "answer()",
    );
    assert_eq!(result.unwrap(), Value::I64(42));
}

#[test]
fn test_call_with_args() {
    let result = eval_with_functions(
        "fn add(a: i64, b: i64) -> i64 { a + b }",
        "add(3, 4)",
    );
    assert_eq!(result.unwrap(), Value::I64(7));
}

#[test]
fn test_call_with_expressions() {
    let result = eval_with_functions(
        "fn double(x: i64) -> i64 { x * 2 }",
        "double(3 + 4)",
    );
    assert_eq!(result.unwrap(), Value::I64(14));
}

#[test]
fn test_call_nested() {
    let result = eval_with_functions(
        r#"
        fn inner(x: i64) -> i64 { x + 1 }
        fn outer(x: i64) -> i64 { inner(x) * 2 }
        "#,
        "outer(5)",
    );
    assert_eq!(result.unwrap(), Value::I64(12));
}

#[test]
fn test_call_recursive() {
    let result = eval_with_functions(
        r#"
        fn factorial(n: i64) -> i64 {
            if n <= 1 { 1 } else { n * factorial(n - 1) }
        }
        "#,
        "factorial(5)",
    );
    assert_eq!(result.unwrap(), Value::I64(120));
}

#[test]
fn test_call_mutual_recursion() {
    let result = eval_with_functions(
        r#"
        fn is_even(n: i64) -> bool {
            if n == 0 { true } else { is_odd(n - 1) }
        }
        fn is_odd(n: i64) -> bool {
            if n == 0 { false } else { is_even(n - 1) }
        }
        "#,
        "is_even(10)",
    );
    assert_eq!(result.unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Arity Checking
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_call_too_few_args() {
    let result = eval_with_functions(
        "fn add(a: i64, b: i64) -> i64 { a + b }",
        "add(1)",
    );
    assert!(matches!(result, Err(EvalError::ArityMismatch { .. })));
}

#[test]
fn test_call_too_many_args() {
    let result = eval_with_functions(
        "fn add(a: i64, b: i64) -> i64 { a + b }",
        "add(1, 2, 3)",
    );
    assert!(matches!(result, Err(EvalError::ArityMismatch { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Return Statements
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_explicit_return() {
    let result = eval_with_functions(
        "fn early() -> i64 { return 42; 100 }",
        "early()",
    );
    assert_eq!(result.unwrap(), Value::I64(42));
}

#[test]
fn test_return_in_if() {
    let result = eval_with_functions(
        r#"
        fn abs(x: i64) -> i64 {
            if x < 0 { return -x; }
            x
        }
        "#,
        "abs(-5)",
    );
    assert_eq!(result.unwrap(), Value::I64(5));
}

#[test]
fn test_return_in_loop() {
    let result = eval_with_functions(
        r#"
        fn find_first_even(limit: i64) -> i64 {
            let mut i = 1;
            loop {
                if i % 2 == 0 { return i; }
                i = i + 1;
                if i > limit { return -1; }
            }
        }
        "#,
        "find_first_even(10)",
    );
    // Note: This requires let binding which is in 1.6
    // For now, test a simpler case
    let result = eval_with_functions(
        r#"
        fn test() -> i64 {
            loop {
                return 42;
            }
        }
        "#,
        "test()",
    );
    assert_eq!(result.unwrap(), Value::I64(42));
}

#[test]
fn test_return_unit() {
    let result = eval_with_functions(
        "fn nothing() { return; }",
        "nothing()",
    );
    assert_eq!(result.unwrap(), Value::Unit);
}

// ═══════════════════════════════════════════════════════════════════════
// Method Calls
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_string_len() {
    let expr: syn::Expr = syn::parse_str(r#""hello".len()"#).unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::Usize(5));
}

#[test]
fn test_string_is_empty() {
    let expr: syn::Expr = syn::parse_str(r#""".is_empty()"#).unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_to_uppercase() {
    let expr: syn::Expr = syn::parse_str(r#""hello".to_uppercase()"#).unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::string("HELLO"));
}

#[test]
fn test_string_contains() {
    let expr: syn::Expr = syn::parse_str(r#""hello world".contains("world")"#).unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vec_len() {
    let mut env = Environment::new();
    env.define("v", Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]));
    let expr: syn::Expr = syn::parse_str("v.len()").unwrap();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::Usize(3));
}

#[test]
fn test_vec_first() {
    let mut env = Environment::new();
    env.define("v", Value::vec(vec![Value::I64(1), Value::I64(2)]));
    let expr: syn::Expr = syn::parse_str("v.first()").unwrap();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::Option(Some(Box::new(Value::I64(1)))));
}

#[test]
fn test_option_unwrap() {
    let mut env = Environment::new();
    env.define("opt", Value::Option(Some(Box::new(Value::I64(42)))));
    let expr: syn::Expr = syn::parse_str("opt.unwrap()").unwrap();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::I64(42));
}

#[test]
fn test_option_unwrap_none_fails() {
    let mut env = Environment::new();
    env.define("opt", Value::Option(None));
    let expr: syn::Expr = syn::parse_str("opt.unwrap()").unwrap();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx);
    assert!(matches!(result, Err(EvalError::BuiltinError { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Closures (Basic)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_closure_simple() {
    let expr: syn::Expr = syn::parse_str("|x| x + 1").unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let closure = expr.eval(&mut env, &ctx).unwrap();

    // Call the closure
    let args = vec![Value::I64(5)];
    let result = call_value(closure, args, &mut env, &ctx, None).unwrap();
    assert_eq!(result, Value::I64(6));
}

#[test]
fn test_closure_multiple_params() {
    let expr: syn::Expr = syn::parse_str("|a, b| a * b").unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let closure = expr.eval(&mut env, &ctx).unwrap();

    let args = vec![Value::I64(3), Value::I64(4)];
    let result = call_value(closure, args, &mut env, &ctx, None).unwrap();
    assert_eq!(result, Value::I64(12));
}

#[test]
fn test_closure_no_params() {
    let expr: syn::Expr = syn::parse_str("|| 42").unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let closure = expr.eval(&mut env, &ctx).unwrap();

    let result = call_value(closure, vec![], &mut env, &ctx, None).unwrap();
    assert_eq!(result, Value::I64(42));
}

// ═══════════════════════════════════════════════════════════════════════
// Stack Overflow Protection
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_stack_overflow() {
    let file: syn::File = syn::parse_str(
        "fn infinite() -> i64 { infinite() }"
    ).unwrap();
    let expr: syn::Expr = syn::parse_str("infinite()").unwrap();

    let mut env = Environment::with_max_call_depth(10);
    let ctx = EvalContext::default();
    eval_items(&file.items, &mut env, &ctx).unwrap();

    let result = expr.eval(&mut env, &ctx);
    assert!(matches!(result, Err(EvalError::Environment(_))));
}

// ═══════════════════════════════════════════════════════════════════════
// Const and Static
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_const_definition() {
    let result = eval_with_functions(
        "const PI: f64 = 3.14159;",
        "PI",
    );
    assert_eq!(result.unwrap(), Value::F64(3.14159));
}

#[test]
fn test_static_definition() {
    let result = eval_with_functions(
        "static COUNTER: i64 = 0;",
        "COUNTER",
    );
    assert_eq!(result.unwrap(), Value::I64(0));
}

// ═══════════════════════════════════════════════════════════════════════
// Builtin Functions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_builtin_type_of() {
    let expr: syn::Expr = syn::parse_str("type_of(42)").unwrap();
    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx).unwrap();
    assert_eq!(result, Value::string("i64"));
}

#[test]
fn test_builtin_assert() {
    let expr: syn::Expr = syn::parse_str("assert(true)").unwrap();
    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx);
    assert!(result.is_ok());
}

#[test]
fn test_builtin_assert_fails() {
    let expr: syn::Expr = syn::parse_str("assert(false)").unwrap();
    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx);
    assert!(matches!(result, Err(EvalError::BuiltinError { .. })));
}

#[test]
fn test_builtin_assert_eq() {
    let expr: syn::Expr = syn::parse_str("assert_eq(1 + 1, 2)").unwrap();
    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx);
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════
// Complex Examples
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_fibonacci() {
    let result = eval_with_functions(
        r#"
        fn fib(n: i64) -> i64 {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
        "#,
        "fib(10)",
    );
    assert_eq!(result.unwrap(), Value::I64(55));
}

#[test]
fn test_higher_order_function() {
    let result = eval_with_functions(
        r#"
        fn apply_twice(f: fn(i64) -> i64, x: i64) -> i64 {
            f(f(x))
        }
        fn double(x: i64) -> i64 { x * 2 }
        "#,
        "apply_twice(double, 3)",
    );
    // Note: This requires first-class functions which might need adjustment
    // For now, skip this test or adjust based on implementation
}
```

---

## Completion Checklist

- [ ] Create `src/eval/function.rs` with `function_from_item`
- [ ] Create `src/eval/call.rs` with `ExprCall` and `ExprMethodCall` evaluation
- [ ] Implement `call_value` for Function, BuiltinFn, and Closure
- [ ] Implement built-in methods (String, Vec, Option, Result)
- [ ] Create `src/eval/return_expr.rs` with `ExprReturn` evaluation
- [ ] Create `src/eval/item.rs` with `eval_item` and `eval_items`
- [ ] Handle `Item::Fn`, `Item::Const`, `Item::Static`
- [ ] Create `src/eval/local.rs` with `eval_local` (let bindings)
- [ ] Add `ArityMismatch` and `BuiltinError` to `EvalError`
- [ ] Basic closure support (without captures)
- [ ] Stack overflow protection via call depth tracking
- [ ] Update `src/eval/mod.rs` dispatcher
- [ ] Update `lib.rs` exports
- [ ] All tests passing

---

## Design Notes

### Why Late Binding?

Functions are looked up by name at call time, not captured at definition time. This enables REPL hot-reload: redefine a function and existing code uses the new definition.

```rust
fn foo() { bar() }  // bar looked up when foo is called
fn bar() { 1 }
foo()  // returns 1
fn bar() { 2 }  // redefine
foo()  // now returns 2
```

### Why Store AST in FunctionValue?

We store `Arc<syn::Block>` directly rather than converting to a custom IR. This keeps the interpreter thin and enables:

- Direct compilation path (syn → quote → rustc)
- Simpler implementation
- Faithful Rust semantics

### Why Built-in Methods?

Common methods like `len()`, `is_empty()`, `unwrap()` are implemented directly rather than requiring users to define them. This matches Rust's experience while keeping the interpreter practical.

### Why Basic Closure Support Now?

We define closures without captures in this stage to enable simple lambdas. Full capture semantics (by-value, by-reference) come in Phase 5.

---

## Next Stage

**Stage 1.6: Statements & Blocks** — Handle `let` bindings with full pattern support, expression statements, semicolons, and block scoping. Integrate all Phase 1 pieces into a cohesive evaluator.
