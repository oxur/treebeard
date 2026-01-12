//! If expression evaluation

use super::Evaluate;
use crate::{Environment, EvalContext, EvalError, Value};

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
        syn::Stmt::Macro(_) => Err(EvalError::UnsupportedExpr {
            kind: "macro statement".to_string(),
            span: None,
        }),
    }
}

fn expr_span(expr: &syn::Expr) -> Option<proc_macro2::Span> {
    use quote::ToTokens;
    expr.to_token_stream().into_iter().next().map(|t| t.span())
}
