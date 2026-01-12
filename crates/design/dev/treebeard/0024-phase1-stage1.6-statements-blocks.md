# Stage 1.6: Statements & Blocks

**Phase:** 1 - Core Evaluator
**Stage:** 1.6 (Final Phase 1 Stage)
**Prerequisites:** Stage 1.1-1.5 (Value, Environment, Expressions, Control Flow, Functions)
**Estimated effort:** 2-3 days

---

## Objective

Complete Phase 1 by implementing full statement evaluation: `let` bindings with patterns, assignment expressions, expression statements, index/field access, tuple/array/struct literals, and comprehensive block scoping. Integrate all Phase 1 pieces into a cohesive evaluator.

---

## Overview

This stage completes the core evaluator with remaining expression and statement types:

| Construct | Example | Notes |
|-----------|---------|-------|
| Let binding | `let x = 1;` `let (a, b) = pair;` | Pattern matching, mutability |
| Assignment | `x = 2;` `arr[0] = 1;` | Requires mutable binding |
| Compound assign | `x += 1;` `x *= 2;` | Desugar to read-op-write |
| Index access | `arr[0]` `map[key]` | Vec, Array, HashMap |
| Field access | `point.x` `person.name` | Struct fields |
| Tuple literal | `(1, 2, 3)` | Creates `Value::Tuple` |
| Array literal | `[1, 2, 3]` | Creates `Value::Array` |
| Struct literal | `Point { x: 1, y: 2 }` | Creates `Value::Struct` |
| Range | `0..10` `0..=9` | Creates iterator for `for` loops |

**Success criteria:** Can evaluate the factorial example from the implementation guide.

---

## File Structure

```
treebeard/src/
├── lib.rs              # Final exports
├── eval/
│   ├── mod.rs          # Complete dispatcher
│   ├── stmt.rs         # ← New: Full statement evaluation
│   ├── assign.rs       # ← New: Assignment expressions
│   ├── index.rs        # ← New: Index expressions
│   ├── field.rs        # ← New: Field access
│   ├── tuple.rs        # ← New: Tuple literals
│   ├── array.rs        # ← New: Array literals
│   ├── struct_lit.rs   # ← New: Struct literals
│   ├── range.rs        # ← New: Range expressions
│   └── local.rs        # Extend from 1.5
└── ...
```

---

## Statement Evaluation

### src/eval/stmt.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;
use super::local::eval_local;
use super::item::eval_item;

/// Evaluate a statement.
pub fn eval_stmt(
    stmt: &syn::Stmt,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    match stmt {
        // Expression without semicolon: value is returned
        syn::Stmt::Expr(expr, None) => expr.eval(env, ctx),

        // Expression with semicolon: evaluate for side effects, return unit
        syn::Stmt::Expr(expr, Some(_)) => {
            expr.eval(env, ctx)?;
            Ok(Value::Unit)
        }

        // Let binding
        syn::Stmt::Local(local) => {
            eval_local(local, env, ctx)?;
            Ok(Value::Unit)
        }

        // Item (fn, struct, etc.) in block
        syn::Stmt::Item(item) => {
            eval_item(item, env, ctx)?;
            Ok(Value::Unit)
        }

        // Macro statement
        syn::Stmt::Macro(stmt_macro) => {
            Err(EvalError::UnsupportedExpr {
                kind: format!("macro statement: {}",
                    stmt_macro.mac.path.segments.last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ),
                span: None,
            })
        }
    }
}

/// Evaluate a block, managing scope.
pub fn eval_block(
    block: &syn::Block,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    env.push_frame();
    let result = eval_block_stmts(&block.stmts, env, ctx);
    env.pop_frame();
    result
}

/// Evaluate statements within a block (without managing scope).
pub fn eval_block_stmts(
    stmts: &[syn::Stmt],
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let mut last_value = Value::Unit;

    for stmt in stmts {
        // Check for interruption
        if ctx.is_interrupted() {
            return Err(EvalError::Interrupted);
        }

        last_value = eval_stmt(stmt, env, ctx)?;
    }

    Ok(last_value)
}
```

---

## Let Bindings (Full Implementation)

### src/eval/local.rs (Complete)

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
    // Evaluate the initializer
    let value = match &local.init {
        Some(init) => {
            // Check for diverging expression (let x = expr else { ... })
            if let Some((_, diverge)) = &init.diverge {
                // let-else pattern
                let init_value = init.expr.eval(env, ctx)?;

                if let Some(bindings) = match_pattern(&local.pat, &init_value, None)? {
                    // Pattern matched - apply bindings
                    let is_mutable = is_pattern_mutable(&local.pat);
                    for (name, val, pat_mut) in bindings {
                        let mode = if is_mutable || pat_mut {
                            BindingMode::Mutable
                        } else {
                            BindingMode::Immutable
                        };
                        env.define_with_mode(&name, val, mode);
                    }
                    return Ok(());
                } else {
                    // Pattern didn't match - evaluate diverging block
                    // This should diverge (return, break, continue, or panic)
                    match diverge.as_ref() {
                        syn::Expr::Block(block) => {
                            super::stmt::eval_block(&block.block, env, ctx)?;
                            // If we get here, the else block didn't diverge
                            return Err(EvalError::NonDivergingLetElse {
                                span: None,
                            });
                        }
                        _ => {
                            diverge.eval(env, ctx)?;
                            return Err(EvalError::NonDivergingLetElse {
                                span: None,
                            });
                        }
                    }
                }
            } else {
                // Normal let binding
                init.expr.eval(env, ctx)?
            }
        }
        None => Value::Unit,
    };

    // Check if the pattern is mutable
    let is_mutable = is_pattern_mutable(&local.pat);

    // Match the pattern
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

/// Check if a pattern has the `mut` keyword at the top level.
fn is_pattern_mutable(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(pat_ident) => pat_ident.mutability.is_some(),
        syn::Pat::Type(pat_type) => is_pattern_mutable(&pat_type.pat),
        _ => false,
    }
}
```

---

## Assignment Expressions

### src/eval/assign.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use crate::error::type_name;
use super::Evaluate;

impl Evaluate for syn::ExprAssign {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate the right-hand side first
        let value = self.right.eval(env, ctx)?;

        // Assign to the left-hand side
        assign_to_expr(&self.left, value, env, ctx)?;

        Ok(Value::Unit)
    }
}

impl Evaluate for syn::ExprAssignOp {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Get current value
        let current = self.left.eval(env, ctx)?;

        // Evaluate right-hand side
        let rhs = self.right.eval(env, ctx)?;

        // Apply the operation
        let new_value = apply_assign_op(&self.op, current, rhs)?;

        // Assign back
        assign_to_expr(&self.left, new_value, env, ctx)?;

        Ok(Value::Unit)
    }
}

/// Assign a value to an expression (lvalue).
fn assign_to_expr(
    expr: &syn::Expr,
    value: Value,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<(), EvalError> {
    match expr {
        // Simple variable assignment: x = value
        syn::Expr::Path(path) => {
            if path.path.segments.len() != 1 {
                return Err(EvalError::UnsupportedExpr {
                    kind: "assignment to qualified path".to_string(),
                    span: None,
                });
            }
            let name = path.path.segments[0].ident.to_string();
            env.assign(&name, value)?;
            Ok(())
        }

        // Index assignment: arr[i] = value
        syn::Expr::Index(index) => {
            assign_to_index(index, value, env, ctx)
        }

        // Field assignment: obj.field = value
        syn::Expr::Field(field) => {
            assign_to_field(field, value, env, ctx)
        }

        // Dereference assignment: *ptr = value
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            Err(EvalError::UnsupportedExpr {
                kind: "dereference assignment (not yet implemented)".to_string(),
                span: None,
            })
        }

        // Parenthesized: (x) = value
        syn::Expr::Paren(paren) => {
            assign_to_expr(&paren.expr, value, env, ctx)
        }

        _ => Err(EvalError::InvalidAssignTarget {
            kind: format!("{:?}", expr),
            span: None,
        }),
    }
}

/// Assign to an index expression.
fn assign_to_index(
    index: &syn::ExprIndex,
    value: Value,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<(), EvalError> {
    // Get the variable name from the base
    let var_name = match index.expr.as_ref() {
        syn::Expr::Path(path) if path.path.segments.len() == 1 => {
            path.path.segments[0].ident.to_string()
        }
        _ => {
            return Err(EvalError::UnsupportedExpr {
                kind: "complex index base in assignment".to_string(),
                span: None,
            });
        }
    };

    // Evaluate the index
    let idx = index.index.eval(env, ctx)?;

    // Get mutable reference to the container
    let container = env.get_mut(&var_name)?.ok_or_else(|| {
        EvalError::UndefinedVariable {
            name: var_name.clone(),
            span: None,
        }
    })?;

    // Perform the assignment
    match container {
        Value::Vec(arc_vec) => {
            let i = idx.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!("Vec index must be usize, got {}", type_name(&idx)),
                span: None,
            })?;

            // We need to get mutable access to the Vec
            let vec = std::sync::Arc::make_mut(arc_vec);
            if i >= vec.len() {
                return Err(EvalError::IndexOutOfBounds {
                    index: i,
                    len: vec.len(),
                    span: None,
                });
            }
            vec[i] = value;
            Ok(())
        }

        Value::Array(arc_arr) => {
            let i = idx.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!("Array index must be usize, got {}", type_name(&idx)),
                span: None,
            })?;

            let arr = std::sync::Arc::make_mut(arc_arr);
            if i >= arr.len() {
                return Err(EvalError::IndexOutOfBounds {
                    index: i,
                    len: arr.len(),
                    span: None,
                });
            }
            arr[i] = value;
            Ok(())
        }

        Value::HashMap(arc_map) => {
            let map = std::sync::Arc::make_mut(arc_map);
            map.insert(idx, value);
            Ok(())
        }

        other => Err(EvalError::TypeError {
            message: format!("cannot index into {}", type_name(other)),
            span: None,
        }),
    }
}

/// Assign to a field expression.
fn assign_to_field(
    field: &syn::ExprField,
    value: Value,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<(), EvalError> {
    // Get the variable name from the base
    let var_name = match field.base.as_ref() {
        syn::Expr::Path(path) if path.path.segments.len() == 1 => {
            path.path.segments[0].ident.to_string()
        }
        _ => {
            return Err(EvalError::UnsupportedExpr {
                kind: "complex field base in assignment".to_string(),
                span: None,
            });
        }
    };

    // Get the field name
    let field_name = match &field.member {
        syn::Member::Named(ident) => ident.to_string(),
        syn::Member::Unnamed(idx) => idx.index.to_string(),
    };

    // Get mutable reference to the struct
    let container = env.get_mut(&var_name)?.ok_or_else(|| {
        EvalError::UndefinedVariable {
            name: var_name.clone(),
            span: None,
        }
    })?;

    match container {
        Value::Struct(arc_struct) => {
            let s = std::sync::Arc::make_mut(arc_struct);
            if s.fields.contains_key(&field_name) {
                s.fields.insert(field_name, value);
                Ok(())
            } else {
                Err(EvalError::UndefinedField {
                    field: field_name,
                    type_name: s.type_name.clone(),
                    span: None,
                })
            }
        }

        Value::Tuple(arc_tuple) => {
            let idx: usize = field_name.parse().map_err(|_| EvalError::TypeError {
                message: format!("invalid tuple index: {}", field_name),
                span: None,
            })?;

            let tuple = std::sync::Arc::make_mut(arc_tuple);
            if idx >= tuple.len() {
                return Err(EvalError::IndexOutOfBounds {
                    index: idx,
                    len: tuple.len(),
                    span: None,
                });
            }
            tuple[idx] = value;
            Ok(())
        }

        other => Err(EvalError::TypeError {
            message: format!("cannot access field on {}", type_name(other)),
            span: None,
        }),
    }
}

/// Apply a compound assignment operator.
fn apply_assign_op(
    op: &syn::BinOp,
    left: Value,
    right: Value,
) -> Result<Value, EvalError> {
    use crate::eval::binary::*;

    let span = None;

    match op {
        syn::BinOp::AddAssign(_) => eval_add(left, right, span),
        syn::BinOp::SubAssign(_) => eval_sub(left, right, span),
        syn::BinOp::MulAssign(_) => eval_mul(left, right, span),
        syn::BinOp::DivAssign(_) => eval_div(left, right, span),
        syn::BinOp::RemAssign(_) => eval_rem(left, right, span),
        syn::BinOp::BitAndAssign(_) => eval_bitand(left, right, span),
        syn::BinOp::BitOrAssign(_) => eval_bitor(left, right, span),
        syn::BinOp::BitXorAssign(_) => eval_bitxor(left, right, span),
        syn::BinOp::ShlAssign(_) => eval_shl(left, right, span),
        syn::BinOp::ShrAssign(_) => eval_shr(left, right, span),
        _ => Err(EvalError::UnsupportedExpr {
            kind: format!("assignment operator: {:?}", op),
            span: None,
        }),
    }
}
```

---

## Index Access

### src/eval/index.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use crate::error::type_name;
use super::Evaluate;

impl Evaluate for syn::ExprIndex {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let base = self.expr.eval(env, ctx)?;
        let index = self.index.eval(env, ctx)?;

        eval_index_access(&base, &index)
    }
}

/// Perform index access on a value.
pub fn eval_index_access(base: &Value, index: &Value) -> Result<Value, EvalError> {
    match base {
        Value::Vec(v) => {
            let i = index.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!("Vec index must be usize, got {}", type_name(index)),
                span: None,
            })?;

            v.get(i).cloned().ok_or_else(|| EvalError::IndexOutOfBounds {
                index: i,
                len: v.len(),
                span: None,
            })
        }

        Value::Array(arr) => {
            let i = index.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!("Array index must be usize, got {}", type_name(index)),
                span: None,
            })?;

            arr.get(i).cloned().ok_or_else(|| EvalError::IndexOutOfBounds {
                index: i,
                len: arr.len(),
                span: None,
            })
        }

        Value::String(s) => {
            let i = index.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!("String index must be usize, got {}", type_name(index)),
                span: None,
            })?;

            // Return the character at index
            s.chars().nth(i).map(Value::Char).ok_or_else(|| EvalError::IndexOutOfBounds {
                index: i,
                len: s.chars().count(),
                span: None,
            })
        }

        Value::HashMap(map) => {
            map.get(index).cloned().ok_or_else(|| EvalError::KeyNotFound {
                key: format!("{:?}", index),
                span: None,
            })
        }

        Value::Tuple(t) => {
            let i = index.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!("Tuple index must be usize, got {}", type_name(index)),
                span: None,
            })?;

            t.get(i).cloned().ok_or_else(|| EvalError::IndexOutOfBounds {
                index: i,
                len: t.len(),
                span: None,
            })
        }

        other => Err(EvalError::TypeError {
            message: format!("cannot index into {}", type_name(other)),
            span: None,
        }),
    }
}
```

---

## Field Access

### src/eval/field.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use crate::error::type_name;
use super::Evaluate;

impl Evaluate for syn::ExprField {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let base = self.base.eval(env, ctx)?;

        let field_name = match &self.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(idx) => idx.index.to_string(),
        };

        eval_field_access(&base, &field_name)
    }
}

/// Access a field on a value.
pub fn eval_field_access(base: &Value, field: &str) -> Result<Value, EvalError> {
    match base {
        Value::Struct(s) => {
            s.fields.get(field).cloned().ok_or_else(|| EvalError::UndefinedField {
                field: field.to_string(),
                type_name: s.type_name.clone(),
                span: None,
            })
        }

        Value::Tuple(t) => {
            let idx: usize = field.parse().map_err(|_| EvalError::TypeError {
                message: format!("invalid tuple field: {}", field),
                span: None,
            })?;

            t.get(idx).cloned().ok_or_else(|| EvalError::IndexOutOfBounds {
                index: idx,
                len: t.len(),
                span: None,
            })
        }

        Value::Enum(e) => {
            // Access struct variant fields
            match &e.data {
                crate::EnumData::Struct(fields) => {
                    fields.get(field).cloned().ok_or_else(|| EvalError::UndefinedField {
                        field: field.to_string(),
                        type_name: format!("{}::{}", e.type_name, e.variant),
                        span: None,
                    })
                }
                crate::EnumData::Tuple(elements) => {
                    let idx: usize = field.parse().map_err(|_| EvalError::TypeError {
                        message: format!("invalid enum tuple field: {}", field),
                        span: None,
                    })?;
                    elements.get(idx).cloned().ok_or_else(|| EvalError::IndexOutOfBounds {
                        index: idx,
                        len: elements.len(),
                        span: None,
                    })
                }
                crate::EnumData::Unit => Err(EvalError::UndefinedField {
                    field: field.to_string(),
                    type_name: format!("{}::{}", e.type_name, e.variant),
                    span: None,
                }),
            }
        }

        other => Err(EvalError::TypeError {
            message: format!("cannot access field `{}` on {}", field, type_name(other)),
            span: None,
        }),
    }
}
```

---

## Tuple Literals

### src/eval/tuple.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;
use std::sync::Arc;

impl Evaluate for syn::ExprTuple {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Empty tuple is unit
        if self.elems.is_empty() {
            return Ok(Value::Unit);
        }

        // Evaluate all elements
        let elements: Vec<Value> = self
            .elems
            .iter()
            .map(|e| e.eval(env, ctx))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Value::Tuple(Arc::new(elements)))
    }
}
```

---

## Array Literals

### src/eval/array.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;
use std::sync::Arc;

impl Evaluate for syn::ExprArray {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let elements: Vec<Value> = self
            .elems
            .iter()
            .map(|e| e.eval(env, ctx))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Value::Array(Arc::new(elements)))
    }
}

impl Evaluate for syn::ExprRepeat {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // [value; count]
        let value = self.expr.eval(env, ctx)?;
        let count = self.len.eval(env, ctx)?;

        let n = count.as_usize().ok_or_else(|| EvalError::TypeError {
            message: format!("array repeat count must be usize"),
            span: None,
        })?;

        let elements: Vec<Value> = std::iter::repeat(value).take(n).collect();
        Ok(Value::Array(Arc::new(elements)))
    }
}
```

---

## Struct Literals

### src/eval/struct_lit.rs

```rust
use crate::{Value, StructValue, Environment, EvalContext, EvalError};
use super::Evaluate;
use std::sync::Arc;
use indexmap::IndexMap;

impl Evaluate for syn::ExprStruct {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Get the struct name from the path
        let type_name = self
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "Anonymous".to_string());

        let mut fields = IndexMap::new();

        // Evaluate each field
        for field in &self.fields {
            let field_name = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(idx) => idx.index.to_string(),
            };

            let value = field.expr.eval(env, ctx)?;
            fields.insert(field_name, value);
        }

        // Handle struct update syntax: Point { x: 1, ..other }
        if let Some(rest) = &self.rest {
            let base = rest.eval(env, ctx)?;

            if let Value::Struct(base_struct) = base {
                // Copy fields from base that aren't overridden
                for (name, value) in base_struct.fields.iter() {
                    if !fields.contains_key(name) {
                        fields.insert(name.clone(), value.clone());
                    }
                }
            } else {
                return Err(EvalError::TypeError {
                    message: format!("struct update syntax requires struct, got {}",
                        crate::error::type_name(&base)),
                    span: None,
                });
            }
        }

        Ok(Value::Struct(Arc::new(StructValue {
            type_name,
            fields,
            is_tuple_struct: false,
        })))
    }
}
```

---

## Range Expressions

### src/eval/range.rs

```rust
use crate::{Value, Environment, EvalContext, EvalError};
use super::Evaluate;
use std::sync::Arc;

impl Evaluate for syn::ExprRange {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let start = self.start.as_ref().map(|e| e.eval(env, ctx)).transpose()?;
        let end = self.end.as_ref().map(|e| e.eval(env, ctx)).transpose()?;

        // Create a range value that can be iterated
        match (&start, &end, &self.limits) {
            // start..end (exclusive)
            (Some(s), Some(e), syn::RangeLimits::HalfOpen(_)) => {
                create_range(s, e, false)
            }
            // start..=end (inclusive)
            (Some(s), Some(e), syn::RangeLimits::Closed(_)) => {
                create_range(s, e, true)
            }
            // ..end
            (None, Some(_), _) => {
                Err(EvalError::UnsupportedExpr {
                    kind: "range from start (..end)".to_string(),
                    span: None,
                })
            }
            // start..
            (Some(_), None, _) => {
                Err(EvalError::UnsupportedExpr {
                    kind: "unbounded range (start..)".to_string(),
                    span: None,
                })
            }
            // ..
            (None, None, _) => {
                Err(EvalError::UnsupportedExpr {
                    kind: "full range (..)".to_string(),
                    span: None,
                })
            }
        }
    }
}

/// Create a range as a Vec of values (for iteration).
fn create_range(start: &Value, end: &Value, inclusive: bool) -> Result<Value, EvalError> {
    match (start, end) {
        (Value::I64(s), Value::I64(e)) => {
            let range: Vec<Value> = if inclusive {
                (*s..=*e).map(Value::I64).collect()
            } else {
                (*s..*e).map(Value::I64).collect()
            };
            Ok(Value::Vec(Arc::new(range)))
        }
        (Value::I32(s), Value::I32(e)) => {
            let range: Vec<Value> = if inclusive {
                (*s..=*e).map(Value::I32).collect()
            } else {
                (*s..*e).map(Value::I32).collect()
            };
            Ok(Value::Vec(Arc::new(range)))
        }
        (Value::Usize(s), Value::Usize(e)) => {
            let range: Vec<Value> = if inclusive {
                (*s..=*e).map(Value::Usize).collect()
            } else {
                (*s..*e).map(Value::Usize).collect()
            };
            Ok(Value::Vec(Arc::new(range)))
        }
        (Value::Char(s), Value::Char(e)) => {
            let range: Vec<Value> = if inclusive {
                (*s..=*e).map(Value::Char).collect()
            } else {
                (*s..*e).map(Value::Char).collect()
            };
            Ok(Value::Vec(Arc::new(range)))
        }
        _ => Err(EvalError::TypeError {
            message: format!(
                "cannot create range from {} to {}",
                crate::error::type_name(start),
                crate::error::type_name(end)
            ),
            span: None,
        }),
    }
}
```

---

## Additional Error Types

### Add to src/error.rs

```rust
/// Invalid assignment target.
#[error("cannot assign to {kind}")]
InvalidAssignTarget {
    kind: String,
    span: Option<Span>,
},

/// Index out of bounds.
#[error("index out of bounds: index {index} >= len {len}")]
IndexOutOfBounds {
    index: usize,
    len: usize,
    span: Option<Span>,
},

/// Key not found in map.
#[error("key not found: {key}")]
KeyNotFound {
    key: String,
    span: Option<Span>,
},

/// Field not found on struct.
#[error("no field `{field}` on type `{type_name}`")]
UndefinedField {
    field: String,
    type_name: String,
    span: Option<Span>,
},

/// Let-else didn't diverge.
#[error("let-else block must diverge (return, break, continue, or panic)")]
NonDivergingLetElse {
    span: Option<Span>,
},
```

---

## Update Dispatcher

### Update src/eval/mod.rs (Complete)

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
pub mod stmt;
pub mod assign;
pub mod index;
pub mod field;
pub mod tuple;
pub mod array;
pub mod struct_lit;
pub mod range;

use crate::{Value, Environment, EvalContext, EvalError};

/// Trait for evaluating AST nodes to values.
pub trait Evaluate {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError>;
}

impl Evaluate for syn::Expr {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        if ctx.is_interrupted() {
            return Err(EvalError::Interrupted);
        }

        match self {
            // Literals and paths
            syn::Expr::Lit(expr) => expr.eval(env, ctx),
            syn::Expr::Path(expr) => expr.eval(env, ctx),

            // Operators
            syn::Expr::Unary(expr) => expr.eval(env, ctx),
            syn::Expr::Binary(expr) => expr.eval(env, ctx),

            // Control flow
            syn::Expr::If(expr) => expr.eval(env, ctx),
            syn::Expr::Match(expr) => expr.eval(env, ctx),
            syn::Expr::Loop(expr) => expr.eval(env, ctx),
            syn::Expr::While(expr) => expr.eval(env, ctx),
            syn::Expr::ForLoop(expr) => expr.eval(env, ctx),
            syn::Expr::Break(expr) => expr.eval(env, ctx),
            syn::Expr::Continue(expr) => expr.eval(env, ctx),

            // Functions
            syn::Expr::Call(expr) => expr.eval(env, ctx),
            syn::Expr::MethodCall(expr) => expr.eval(env, ctx),
            syn::Expr::Return(expr) => expr.eval(env, ctx),
            syn::Expr::Closure(expr) => eval_closure_expr(expr, env, ctx),

            // Blocks
            syn::Expr::Block(expr) => stmt::eval_block(&expr.block, env, ctx),

            // Assignment
            syn::Expr::Assign(expr) => expr.eval(env, ctx),
            syn::Expr::AssignOp(expr) => expr.eval(env, ctx),

            // Access
            syn::Expr::Index(expr) => expr.eval(env, ctx),
            syn::Expr::Field(expr) => expr.eval(env, ctx),

            // Compound literals
            syn::Expr::Tuple(expr) => expr.eval(env, ctx),
            syn::Expr::Array(expr) => expr.eval(env, ctx),
            syn::Expr::Repeat(expr) => expr.eval(env, ctx),
            syn::Expr::Struct(expr) => expr.eval(env, ctx),
            syn::Expr::Range(expr) => expr.eval(env, ctx),

            // Grouping
            syn::Expr::Paren(expr) => expr.expr.eval(env, ctx),
            syn::Expr::Group(expr) => expr.expr.eval(env, ctx),

            // Reference (basic support)
            syn::Expr::Reference(expr) => {
                // For now, just evaluate the inner expression
                // Full reference semantics in Phase 5
                expr.expr.eval(env, ctx)
            }

            // Cast (basic support)
            syn::Expr::Cast(expr) => {
                // For now, just evaluate the expression
                // Type casting can be added later
                expr.expr.eval(env, ctx)
            }

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

fn expr_span(expr: &syn::Expr) -> proc_macro2::Span {
    use quote::ToTokens;
    expr.to_token_stream()
        .into_iter()
        .next()
        .map(|t| t.span())
        .unwrap_or_else(proc_macro2::Span::call_site)
}

fn eval_closure_expr(
    expr: &syn::ExprClosure,
    _env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<Value, EvalError> {
    use std::sync::Arc;
    use crate::ClosureValue;

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

    let body = expr.body.as_ref().clone();

    Ok(Value::Closure(Arc::new(ClosureValue {
        params,
        body: Arc::new(body),
        captures: Arc::new(vec![]),
    })))
}

// Re-exports
pub use stmt::{eval_stmt, eval_block};
pub use pattern::{match_pattern, apply_bindings};
pub use item::{eval_item, eval_items};
pub use call::call_value;
pub use local::eval_local;
pub use control::ControlFlow;
```

---

## Update lib.rs (Final Exports)

```rust
pub mod value;
pub mod environment;
pub mod context;
pub mod error;
pub mod eval;

// Value types
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

// Environment
pub use environment::{Environment, Binding, BindingMode, ScopeGuard};

// Context
pub use context::EvalContext;

// Errors
pub use error::{TreebeardError, EnvironmentError, EvalError};

// Evaluation
pub use eval::{
    Evaluate,
    eval_stmt,
    eval_block,
    eval_item,
    eval_items,
    eval_expr,
    call_value,
    ControlFlow,
};

/// Convenience function to evaluate a Rust source string.
pub fn eval_str(source: &str) -> Result<Value, EvalError> {
    let file: syn::File = syn::parse_str(source)
        .map_err(|e| EvalError::ParseError {
            message: e.to_string(),
            span: None,
        })?;

    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();

    eval_items(&file.items, &mut env, &ctx)
}

/// Evaluate a source string and then an expression.
pub fn eval_with_expr(items_source: &str, expr_source: &str) -> Result<Value, EvalError> {
    let file: syn::File = syn::parse_str(items_source)
        .map_err(|e| EvalError::ParseError {
            message: e.to_string(),
            span: None,
        })?;

    let expr: syn::Expr = syn::parse_str(expr_source)
        .map_err(|e| EvalError::ParseError {
            message: e.to_string(),
            span: None,
        })?;

    let mut env = Environment::with_prelude();
    let ctx = EvalContext::default();

    eval_items(&file.items, &mut env, &ctx)?;
    expr.eval(&mut env, &ctx)
}
```

---

## Test Cases

### tests/stmt_tests.rs

```rust
use treebeard_core::*;

// ═══════════════════════════════════════════════════════════════════════
// Let Bindings
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_let_simple() {
    let result = eval_with_expr("", "{ let x = 42; x }");
    assert_eq!(result.unwrap(), Value::I64(42));
}

#[test]
fn test_let_mutable() {
    let result = eval_with_expr("", "{ let mut x = 1; x = 2; x }");
    assert_eq!(result.unwrap(), Value::I64(2));
}

#[test]
fn test_let_pattern_tuple() {
    let result = eval_with_expr("", "{ let (a, b) = (1, 2); a + b }");
    assert_eq!(result.unwrap(), Value::I64(3));
}

#[test]
fn test_let_pattern_nested() {
    let result = eval_with_expr("", "{ let ((a, b), c) = ((1, 2), 3); a + b + c }");
    assert_eq!(result.unwrap(), Value::I64(6));
}

#[test]
fn test_let_shadowing() {
    let result = eval_with_expr("", "{ let x = 1; let x = 2; x }");
    assert_eq!(result.unwrap(), Value::I64(2));
}

#[test]
fn test_let_type_annotation() {
    let result = eval_with_expr("", "{ let x: i64 = 42; x }");
    assert_eq!(result.unwrap(), Value::I64(42));
}

// ═══════════════════════════════════════════════════════════════════════
// Assignment
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_assign_simple() {
    let result = eval_with_expr("", "{ let mut x = 1; x = 42; x }");
    assert_eq!(result.unwrap(), Value::I64(42));
}

#[test]
fn test_assign_immutable_fails() {
    let result = eval_with_expr("", "{ let x = 1; x = 2; x }");
    assert!(result.is_err());
}

#[test]
fn test_assign_compound_add() {
    let result = eval_with_expr("", "{ let mut x = 10; x += 5; x }");
    assert_eq!(result.unwrap(), Value::I64(15));
}

#[test]
fn test_assign_compound_sub() {
    let result = eval_with_expr("", "{ let mut x = 10; x -= 3; x }");
    assert_eq!(result.unwrap(), Value::I64(7));
}

#[test]
fn test_assign_compound_mul() {
    let result = eval_with_expr("", "{ let mut x = 5; x *= 4; x }");
    assert_eq!(result.unwrap(), Value::I64(20));
}

#[test]
fn test_assign_index() {
    let result = eval_with_expr("", "{ let mut arr = [1, 2, 3]; arr[1] = 42; arr[1] }");
    assert_eq!(result.unwrap(), Value::I64(42));
}

#[test]
fn test_assign_field() {
    let result = eval_with_expr(
        "struct Point { x: i64, y: i64 }",
        "{ let mut p = Point { x: 1, y: 2 }; p.x = 10; p.x }",
    );
    assert_eq!(result.unwrap(), Value::I64(10));
}

// ═══════════════════════════════════════════════════════════════════════
// Index Access
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_index_array() {
    let result = eval_with_expr("", "{ let arr = [10, 20, 30]; arr[1] }");
    assert_eq!(result.unwrap(), Value::I64(20));
}

#[test]
fn test_index_vec() {
    let result = eval_with_expr("", r#"{ let v = [1, 2, 3]; v[0] }"#);
    assert_eq!(result.unwrap(), Value::I64(1));
}

#[test]
fn test_index_string() {
    let result = eval_with_expr("", r#"{ let s = "hello"; s[1] }"#);
    assert_eq!(result.unwrap(), Value::Char('e'));
}

#[test]
fn test_index_out_of_bounds() {
    let result = eval_with_expr("", "{ let arr = [1, 2, 3]; arr[10] }");
    assert!(matches!(result, Err(EvalError::IndexOutOfBounds { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Field Access
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_field_struct() {
    let result = eval_with_expr(
        "struct Point { x: i64, y: i64 }",
        "{ let p = Point { x: 3, y: 4 }; p.x + p.y }",
    );
    assert_eq!(result.unwrap(), Value::I64(7));
}

#[test]
fn test_field_tuple() {
    let result = eval_with_expr("", "{ let t = (1, 2, 3); t.1 }");
    assert_eq!(result.unwrap(), Value::I64(2));
}

#[test]
fn test_field_undefined() {
    let result = eval_with_expr(
        "struct Point { x: i64, y: i64 }",
        "{ let p = Point { x: 1, y: 2 }; p.z }",
    );
    assert!(matches!(result, Err(EvalError::UndefinedField { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Tuple Literals
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_tuple_empty() {
    let result = eval_with_expr("", "()");
    assert_eq!(result.unwrap(), Value::Unit);
}

#[test]
fn test_tuple_simple() {
    let result = eval_with_expr("", "(1, 2, 3)");
    assert_eq!(
        result.unwrap(),
        Value::Tuple(std::sync::Arc::new(vec![
            Value::I64(1),
            Value::I64(2),
            Value::I64(3)
        ]))
    );
}

#[test]
fn test_tuple_nested() {
    let result = eval_with_expr("", "((1, 2), (3, 4))");
    assert!(matches!(result.unwrap(), Value::Tuple(_)));
}

// ═══════════════════════════════════════════════════════════════════════
// Array Literals
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_array_literal() {
    let result = eval_with_expr("", "[1, 2, 3]");
    assert!(matches!(result.unwrap(), Value::Array(_)));
}

#[test]
fn test_array_repeat() {
    let result = eval_with_expr("", "[0; 5]");
    let arr = result.unwrap();
    if let Value::Array(a) = arr {
        assert_eq!(a.len(), 5);
        assert!(a.iter().all(|v| *v == Value::I64(0)));
    } else {
        panic!("Expected array");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Struct Literals
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_struct_literal() {
    let result = eval_with_expr(
        "struct Point { x: i64, y: i64 }",
        "Point { x: 1, y: 2 }",
    );
    assert!(matches!(result.unwrap(), Value::Struct(_)));
}

#[test]
fn test_struct_update_syntax() {
    let result = eval_with_expr(
        "struct Point { x: i64, y: i64 }",
        "{ let p1 = Point { x: 1, y: 2 }; let p2 = Point { x: 10, ..p1 }; p2.y }",
    );
    assert_eq!(result.unwrap(), Value::I64(2));
}

// ═══════════════════════════════════════════════════════════════════════
// Range Expressions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_range_exclusive() {
    let result = eval_with_expr("", "{ let r = 0..5; r.len() }");
    assert_eq!(result.unwrap(), Value::Usize(5));
}

#[test]
fn test_range_inclusive() {
    let result = eval_with_expr("", "{ let r = 0..=4; r.len() }");
    assert_eq!(result.unwrap(), Value::Usize(5));
}

#[test]
fn test_range_in_for() {
    let result = eval_with_expr(
        r#"
        fn sum_range() -> i64 {
            let mut total = 0;
            for i in 1..=5 {
                total += i;
            }
            total
        }
        "#,
        "sum_range()",
    );
    assert_eq!(result.unwrap(), Value::I64(15));
}

// ═══════════════════════════════════════════════════════════════════════
// Block Scoping
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_block_scope() {
    let result = eval_with_expr("", "{ let x = 1; { let x = 2; x } }");
    assert_eq!(result.unwrap(), Value::I64(2));
}

#[test]
fn test_block_scope_outer() {
    let result = eval_with_expr("", "{ let x = 1; { let _y = 2; }; x }");
    assert_eq!(result.unwrap(), Value::I64(1));
}

#[test]
fn test_block_returns_last() {
    let result = eval_with_expr("", "{ 1; 2; 3 }");
    assert_eq!(result.unwrap(), Value::I64(3));
}

#[test]
fn test_block_semicolon_unit() {
    let result = eval_with_expr("", "{ 1; 2; 3; }");
    assert_eq!(result.unwrap(), Value::Unit);
}

// ═══════════════════════════════════════════════════════════════════════
// Phase 1 Integration Tests (Success Criteria)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_factorial() {
    let result = eval_with_expr(
        r#"
        fn factorial(n: i64) -> i64 {
            if n <= 1 {
                1
            } else {
                n * factorial(n - 1)
            }
        }
        "#,
        "factorial(5)",
    );
    assert_eq!(result.unwrap(), Value::I64(120));
}

#[test]
fn test_fibonacci() {
    let result = eval_with_expr(
        r#"
        fn fib(n: i64) -> i64 {
            if n <= 1 {
                n
            } else {
                fib(n - 1) + fib(n - 2)
            }
        }
        "#,
        "fib(10)",
    );
    assert_eq!(result.unwrap(), Value::I64(55));
}

#[test]
fn test_iterative_sum() {
    let result = eval_with_expr(
        r#"
        fn sum_to(n: i64) -> i64 {
            let mut total = 0;
            let mut i = 1;
            while i <= n {
                total += i;
                i += 1;
            }
            total
        }
        "#,
        "sum_to(100)",
    );
    assert_eq!(result.unwrap(), Value::I64(5050));
}

#[test]
fn test_for_loop_sum() {
    let result = eval_with_expr(
        r#"
        fn sum_array(arr: [i64; 5]) -> i64 {
            let mut total = 0;
            for x in arr {
                total += x;
            }
            total
        }
        "#,
        "sum_array([1, 2, 3, 4, 5])",
    );
    assert_eq!(result.unwrap(), Value::I64(15));
}

#[test]
fn test_match_expression() {
    let result = eval_with_expr(
        r#"
        fn describe(n: i64) -> i64 {
            match n {
                0 => 0,
                1 => 1,
                n if n < 0 => -1,
                _ => 2,
            }
        }
        "#,
        "describe(42)",
    );
    assert_eq!(result.unwrap(), Value::I64(2));
}

#[test]
fn test_nested_functions() {
    let result = eval_with_expr(
        r#"
        fn outer(x: i64) -> i64 {
            fn inner(y: i64) -> i64 {
                y * 2
            }
            inner(x) + 1
        }
        "#,
        "outer(5)",
    );
    assert_eq!(result.unwrap(), Value::I64(11));
}

#[test]
fn test_complex_expression() {
    let result = eval_with_expr(
        r#"
        struct Point { x: i64, y: i64 }

        fn distance_squared(p: Point) -> i64 {
            p.x * p.x + p.y * p.y
        }
        "#,
        "distance_squared(Point { x: 3, y: 4 })",
    );
    assert_eq!(result.unwrap(), Value::I64(25));
}

#[test]
fn test_early_return() {
    let result = eval_with_expr(
        r#"
        fn find_first_even(arr: [i64; 5]) -> i64 {
            for x in arr {
                if x % 2 == 0 {
                    return x;
                }
            }
            -1
        }
        "#,
        "find_first_even([1, 3, 4, 5, 6])",
    );
    assert_eq!(result.unwrap(), Value::I64(4));
}

#[test]
fn test_all_control_flow() {
    let result = eval_with_expr(
        r#"
        fn complex() -> i64 {
            let mut result = 0;

            // for loop
            for i in 1..=3 {
                result += i;
            }

            // while loop with break
            let mut j = 0;
            while true {
                j += 1;
                if j >= 5 {
                    break;
                }
            }
            result += j;

            // match expression
            result += match result {
                n if n > 10 => 100,
                _ => 0,
            };

            result
        }
        "#,
        "complex()",
    );
    assert_eq!(result.unwrap(), Value::I64(111)); // 6 + 5 + 100
}
```

---

## Completion Checklist

- [ ] Create `src/eval/stmt.rs` with `eval_stmt` and `eval_block`
- [ ] Complete `src/eval/local.rs` with full let binding support
- [ ] Handle `let-else` pattern
- [ ] Create `src/eval/assign.rs` with assignment expressions
- [ ] Implement compound assignment operators (`+=`, `-=`, etc.)
- [ ] Handle index assignment (`arr[i] = x`)
- [ ] Handle field assignment (`s.field = x`)
- [ ] Create `src/eval/index.rs` with index access
- [ ] Create `src/eval/field.rs` with field access
- [ ] Create `src/eval/tuple.rs` with tuple literals
- [ ] Create `src/eval/array.rs` with array literals and repeat
- [ ] Create `src/eval/struct_lit.rs` with struct literals
- [ ] Handle struct update syntax (`Point { x: 1, ..other }`)
- [ ] Create `src/eval/range.rs` with range expressions
- [ ] Add new error types to `error.rs`
- [ ] Update `src/eval/mod.rs` dispatcher (complete)
- [ ] Add convenience functions to `lib.rs`
- [ ] All Phase 1 integration tests passing
- [ ] Factorial example works ✓

---

## Design Notes

### Why Ranges Expand to Vec?

For simplicity, ranges like `0..10` are eagerly expanded to `Vec<Value>`. This allows reuse of the existing `for` loop iteration. A lazy range iterator can be added later if needed for performance.

### Why Arc::make_mut for Assignment?

`Value` uses `Arc` for heap types to enable cheap cloning. When assigning to a Vec/Array element or struct field, we use `Arc::make_mut` to get exclusive mutable access, cloning only if there are other references.

### Why Blocks Manage Their Own Scope?

Each block (`{ ... }`) pushes a new frame and pops it when done. This ensures variables declared in a block don't leak out, matching Rust semantics exactly.

### Phase 1 Complete

With this stage, the core evaluator can:

- ✅ Evaluate all primitive literals
- ✅ Look up variables
- ✅ Perform arithmetic, comparison, logical, bitwise operations
- ✅ Handle `if`/`else` and `match`
- ✅ Execute `loop`, `while`, `for` with `break`/`continue`
- ✅ Define and call functions with recursion
- ✅ Bind variables with patterns
- ✅ Assign to variables, indices, and fields
- ✅ Create tuples, arrays, structs
- ✅ Use ranges in `for` loops

---

## Next Phase

**Phase 2: Frontend Trait** — Define the `LanguageFrontend` abstraction that allows multiple syntaxes (Rust, Oxur) to target Treebeard.
