//! Array literal evaluation

use crate::{EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate an array literal expression.
///
/// # Examples
///
/// - `[1, 2, 3]` → Array with three elements
/// - `[0; 5]` → Array with five zeros (repeat syntax)
///
/// # Errors
///
/// Returns errors from evaluating array elements.
/// Returns `TypeError` if repeat count is not an integer.
pub fn eval_array(
    array: &syn::ExprArray,
    env: &mut crate::Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let elements = array
        .elems
        .iter()
        .map(|elem| elem.eval(env, ctx))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Value::array(elements))
}

/// Evaluate an array repeat expression `[value; count]`.
///
/// # Errors
///
/// Returns `TypeError` if count is not an integer.
pub fn eval_array_repeat(
    repeat: &syn::ExprRepeat,
    env: &mut crate::Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Evaluate the value to repeat
    let value = repeat.expr.eval(env, ctx)?;

    // Evaluate the count
    let count_val = repeat.len.eval(env, ctx)?;
    let count = count_val.as_usize().ok_or_else(|| EvalError::TypeError {
        message: format!(
            "array repeat count must be integer, got {}",
            crate::error::type_name(&count_val)
        ),
        span: None,
    })?;

    // Create array with repeated value
    let elements = vec![value; count];

    Ok(Value::array(elements))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;

    #[test]
    fn test_empty_array() {
        let expr: syn::Expr = syn::parse_str("[]").unwrap();
        if let syn::Expr::Array(array) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_array(&array, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::array(vec![]));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_array_with_elements() {
        let expr: syn::Expr = syn::parse_str("[1, 2, 3]").unwrap();
        if let syn::Expr::Array(array) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_array(&array, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::array(vec![Value::I64(1), Value::I64(2), Value::I64(3)])
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_array_with_expressions() {
        let expr: syn::Expr = syn::parse_str("[1 + 1, 2 * 2, 3 - 1]").unwrap();
        if let syn::Expr::Array(array) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_array(&array, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::array(vec![Value::I64(2), Value::I64(4), Value::I64(2)])
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_array_repeat_syntax() {
        let expr: syn::Expr = syn::parse_str("[0; 5]").unwrap();
        if let syn::Expr::Repeat(repeat) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_array_repeat(&repeat, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::array(vec![
                    Value::I64(0),
                    Value::I64(0),
                    Value::I64(0),
                    Value::I64(0),
                    Value::I64(0)
                ])
            );
        } else {
            panic!("Expected Repeat");
        }
    }

    #[test]
    fn test_array_repeat_with_expression() {
        let expr: syn::Expr = syn::parse_str("[42; 3]").unwrap();
        if let syn::Expr::Repeat(repeat) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_array_repeat(&repeat, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::array(vec![Value::I64(42), Value::I64(42), Value::I64(42)])
            );
        } else {
            panic!("Expected Repeat");
        }
    }

    #[test]
    fn test_array_repeat_computed_count() {
        let expr: syn::Expr = syn::parse_str("[x; n]").unwrap();
        if let syn::Expr::Repeat(repeat) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define("x".to_string(), Value::I64(7));
            env.define("n".to_string(), Value::I64(4));

            let result = eval_array_repeat(&repeat, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::array(vec![
                    Value::I64(7),
                    Value::I64(7),
                    Value::I64(7),
                    Value::I64(7)
                ])
            );
        } else {
            panic!("Expected Repeat");
        }
    }

    #[test]
    fn test_nested_arrays() {
        let expr: syn::Expr = syn::parse_str("[[1, 2], [3, 4]]").unwrap();
        if let syn::Expr::Array(array) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_array(&array, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::array(vec![
                    Value::array(vec![Value::I64(1), Value::I64(2)]),
                    Value::array(vec![Value::I64(3), Value::I64(4)])
                ])
            );
        } else {
            panic!("Expected Array");
        }
    }
}
