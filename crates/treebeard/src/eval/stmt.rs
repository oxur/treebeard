//! Statement evaluation

use crate::{Environment, EvalContext, EvalError, Value};

use super::item::eval_item;
use super::local::eval_local;
use super::Evaluate;

/// Evaluate a statement.
///
/// # Errors
///
/// Returns errors from statement evaluation.
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
        syn::Stmt::Macro(stmt_macro) => Err(EvalError::UnsupportedExpr {
            kind: format!(
                "macro statement: {}",
                stmt_macro
                    .mac
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            span: None,
        }),
    }
}

/// Evaluate a block, managing scope.
///
/// Creates a new frame, evaluates statements, then pops the frame.
///
/// # Errors
///
/// Returns errors from statement evaluation.
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
///
/// # Errors
///
/// Returns errors from statement evaluation.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_stmt_expr_no_semi() {
        // Parse as a block to get a statement without semicolon
        let block: syn::Block = syn::parse_str("{ 42 }").unwrap();
        let stmt = &block.stmts[0];
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_stmt(stmt, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_eval_stmt_expr_with_semi() {
        let stmt: syn::Stmt = syn::parse_str("42;").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_stmt(&stmt, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_eval_block_returns_last() {
        let block: syn::Block = syn::parse_str("{ 1; 2; 3 }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    #[test]
    fn test_eval_block_semi_returns_unit() {
        let block: syn::Block = syn::parse_str("{ 1; 2; 3; }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_eval_block_scope() {
        let block: syn::Block = syn::parse_str("{ let x = 42; x }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));

        // x should not be in scope after block
        assert!(env.get("x").is_none());
    }
}
