//! Assignment expression evaluation

use crate::{Environment, EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate an assignment expression.
///
/// Handles simple assignment (=).
///
/// # Errors
///
/// Returns `InvalidAssignTarget` if the left side is not a valid assignment target.
pub fn eval_assign(
    assign: &syn::ExprAssign,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let right_value = assign.right.eval(env, ctx)?;

    assign_to_expr(&assign.left, right_value, env, ctx)?;

    Ok(Value::Unit)
}

/// Assign a value to an expression (lvalue).
///
/// # Errors
///
/// Returns `InvalidAssignTarget` for unsupported assignment targets.
fn assign_to_expr(
    target: &syn::Expr,
    value: Value,
    env: &mut Environment,
    _ctx: &EvalContext,
) -> Result<(), EvalError> {
    match target {
        // Simple variable assignment
        syn::Expr::Path(path) => {
            let name = path_to_string(path);
            env.assign(&name, value).map_err(EvalError::from)?;
            Ok(())
        }

        // Index assignment: vec[i] = value
        syn::Expr::Index(_index_expr) => {
            // Index assignment requires mutable reference tracking
            // Will be implemented in src/eval/index.rs
            Err(EvalError::UnsupportedExpr {
                kind: "index assignment (use index module)".to_string(),
                span: None,
            })
        }

        // Field assignment: struct.field = value
        syn::Expr::Field(_field_expr) => {
            // Field assignment requires mutable reference tracking
            // Will be implemented in src/eval/field.rs
            Err(EvalError::UnsupportedExpr {
                kind: "field assignment (use field module)".to_string(),
                span: None,
            })
        }

        // Invalid assignment target
        _ => Err(EvalError::InvalidAssignTarget {
            kind: format!("{:?}", target),
            span: None,
        }),
    }
}

/// Convert a path to a string identifier.
fn path_to_string(path: &syn::ExprPath) -> String {
    path.path
        .segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_assignment() {
        let expr: syn::Expr = syn::parse_str("x = 42").unwrap();
        if let syn::Expr::Assign(assign) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            // First define x as mutable
            env.define_with_mode("x".to_string(), Value::I64(0), crate::BindingMode::Mutable);

            let result = eval_assign(&assign, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::Unit);
            assert_eq!(env.get("x"), Some(&Value::I64(42)));
        } else {
            panic!("Expected Assign");
        }
    }

    #[test]
    fn test_assignment_to_immutable_fails() {
        let expr: syn::Expr = syn::parse_str("x = 42").unwrap();
        if let syn::Expr::Assign(assign) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            // Define x as immutable
            env.define("x".to_string(), Value::I64(0));

            let result = eval_assign(&assign, &mut env, &ctx);
            assert!(result.is_err());
        } else {
            panic!("Expected Assign");
        }
    }

    #[test]
    fn test_assignment_to_undefined_fails() {
        let expr: syn::Expr = syn::parse_str("x = 42").unwrap();
        if let syn::Expr::Assign(assign) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_assign(&assign, &mut env, &ctx);
            assert!(result.is_err());
        } else {
            panic!("Expected Assign");
        }
    }
}
