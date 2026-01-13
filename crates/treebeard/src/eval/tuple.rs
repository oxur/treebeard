//! Tuple literal evaluation

use crate::{EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate a tuple literal expression.
///
/// # Examples
///
/// - `()` → Unit
/// - `(1,)` → Single element tuple
/// - `(1, 2, 3)` → Three element tuple
///
/// # Errors
///
/// Returns errors from evaluating tuple elements.
pub fn eval_tuple(
    tuple: &syn::ExprTuple,
    env: &mut crate::Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Empty tuple is Unit
    if tuple.elems.is_empty() {
        return Ok(Value::Unit);
    }

    // Evaluate all elements
    let mut elements = Vec::with_capacity(tuple.elems.len());
    for elem in &tuple.elems {
        elements.push(elem.eval(env, ctx)?);
    }

    Ok(Value::tuple(elements))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;

    #[test]
    fn test_empty_tuple() {
        let expr: syn::Expr = syn::parse_str("()").unwrap();
        if let syn::Expr::Tuple(tuple) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_tuple(&tuple, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::Unit);
        } else {
            panic!("Expected Tuple");
        }
    }

    #[test]
    fn test_single_element_tuple() {
        let expr: syn::Expr = syn::parse_str("(42,)").unwrap();
        if let syn::Expr::Tuple(tuple) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_tuple(&tuple, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::tuple(vec![Value::I64(42)]));
        } else {
            panic!("Expected Tuple");
        }
    }

    #[test]
    fn test_two_element_tuple() {
        let expr: syn::Expr = syn::parse_str("(1, 2)").unwrap();
        if let syn::Expr::Tuple(tuple) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_tuple(&tuple, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::tuple(vec![Value::I64(1), Value::I64(2)]));
        } else {
            panic!("Expected Tuple");
        }
    }

    #[test]
    fn test_mixed_type_tuple() {
        let expr: syn::Expr = syn::parse_str(r#"(42, "hello", true)"#).unwrap();
        if let syn::Expr::Tuple(tuple) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_tuple(&tuple, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::tuple(vec![
                    Value::I64(42),
                    Value::string("hello"),
                    Value::Bool(true)
                ])
            );
        } else {
            panic!("Expected Tuple");
        }
    }

    #[test]
    fn test_nested_tuple() {
        let expr: syn::Expr = syn::parse_str("((1, 2), (3, 4))").unwrap();
        if let syn::Expr::Tuple(tuple) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_tuple(&tuple, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::tuple(vec![
                    Value::tuple(vec![Value::I64(1), Value::I64(2)]),
                    Value::tuple(vec![Value::I64(3), Value::I64(4)])
                ])
            );
        } else {
            panic!("Expected Tuple");
        }
    }

    #[test]
    fn test_tuple_with_expressions() {
        let expr: syn::Expr = syn::parse_str("(1 + 2, 3 * 4)").unwrap();
        if let syn::Expr::Tuple(tuple) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_tuple(&tuple, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::tuple(vec![Value::I64(3), Value::I64(12)]));
        } else {
            panic!("Expected Tuple");
        }
    }
}
