//! Range expression evaluation

use crate::{EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate a range expression.
///
/// Ranges are eagerly expanded to Vec for iteration.
///
/// # Examples
///
/// - `0..5` → Vec [0, 1, 2, 3, 4]
/// - `0..=5` → Vec [0, 1, 2, 3, 4, 5]
/// - `5..` → Unbounded from (not yet supported)
/// - `..5` → Unbounded to (not yet supported)
/// - `..` → Full range (not yet supported)
///
/// # Errors
///
/// Returns `TypeError` if range bounds are not integers.
/// Returns `UnsupportedExpr` for unbounded ranges.
pub fn eval_range(
    range: &syn::ExprRange,
    env: &mut crate::Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    use syn::RangeLimits;

    match (&range.start, &range.end, &range.limits) {
        // Bounded range: start..end or start..=end
        (Some(start_expr), Some(end_expr), limits) => {
            let start = start_expr.eval(env, ctx)?;
            let end = end_expr.eval(env, ctx)?;

            let start_i64 = start.as_i64().ok_or_else(|| EvalError::TypeError {
                message: format!(
                    "range start must be integer, got {}",
                    crate::error::type_name(&start)
                ),
                span: None,
            })?;

            let end_i64 = end.as_i64().ok_or_else(|| EvalError::TypeError {
                message: format!(
                    "range end must be integer, got {}",
                    crate::error::type_name(&end)
                ),
                span: None,
            })?;

            // Generate the range values
            let values = match limits {
                RangeLimits::HalfOpen(_) => {
                    // start..end (exclusive)
                    (start_i64..end_i64).map(Value::I64).collect()
                }
                RangeLimits::Closed(_) => {
                    // start..=end (inclusive)
                    (start_i64..=end_i64).map(Value::I64).collect()
                }
            };

            Ok(Value::vec(values))
        }

        // Unbounded ranges (not yet supported)
        _ => Err(EvalError::UnsupportedExpr {
            kind: "unbounded range (use bounded ranges like 0..10)".to_string(),
            span: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;

    #[test]
    fn test_range_exclusive() {
        let expr: syn::Expr = syn::parse_str("0..5").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::vec(vec![
                    Value::I64(0),
                    Value::I64(1),
                    Value::I64(2),
                    Value::I64(3),
                    Value::I64(4)
                ])
            );
        } else {
            panic!("Expected Range");
        }
    }

    #[test]
    fn test_range_inclusive() {
        let expr: syn::Expr = syn::parse_str("0..=5").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::vec(vec![
                    Value::I64(0),
                    Value::I64(1),
                    Value::I64(2),
                    Value::I64(3),
                    Value::I64(4),
                    Value::I64(5)
                ])
            );
        } else {
            panic!("Expected Range");
        }
    }

    #[test]
    fn test_range_with_variables() {
        let expr: syn::Expr = syn::parse_str("start..end").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define("start".to_string(), Value::I64(10));
            env.define("end".to_string(), Value::I64(13));

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::vec(vec![Value::I64(10), Value::I64(11), Value::I64(12)])
            );
        } else {
            panic!("Expected Range");
        }
    }

    #[test]
    fn test_range_with_expressions() {
        let expr: syn::Expr = syn::parse_str("1+1..2*3").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::vec(vec![
                    Value::I64(2),
                    Value::I64(3),
                    Value::I64(4),
                    Value::I64(5)
                ])
            );
        } else {
            panic!("Expected Range");
        }
    }

    #[test]
    fn test_range_empty() {
        let expr: syn::Expr = syn::parse_str("5..5").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::vec(vec![]));
        } else {
            panic!("Expected Range");
        }
    }

    #[test]
    fn test_range_reverse() {
        let expr: syn::Expr = syn::parse_str("5..2").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            // Rust ranges with start > end produce empty ranges
            assert_eq!(result, Value::vec(vec![]));
        } else {
            panic!("Expected Range");
        }
    }

    #[test]
    fn test_range_single_element() {
        let expr: syn::Expr = syn::parse_str("5..=5").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::vec(vec![Value::I64(5)]));
        } else {
            panic!("Expected Range");
        }
    }

    #[test]
    fn test_range_negative_values() {
        let expr: syn::Expr = syn::parse_str("-3..2").unwrap();
        if let syn::Expr::Range(range) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_range(&range, &mut env, &ctx).unwrap();
            assert_eq!(
                result,
                Value::vec(vec![
                    Value::I64(-3),
                    Value::I64(-2),
                    Value::I64(-1),
                    Value::I64(0),
                    Value::I64(1)
                ])
            );
        } else {
            panic!("Expected Range");
        }
    }
}
