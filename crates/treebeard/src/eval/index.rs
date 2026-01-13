//! Index expression evaluation

use crate::{EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate an index expression.
///
/// Supports indexing into Vec, Array, String, and HashMap.
///
/// # Errors
///
/// Returns `IndexOutOfBounds` if the index is out of range.
/// Returns `KeyNotFound` if the key doesn't exist in a HashMap.
/// Returns `TypeError` if the base value doesn't support indexing.
pub fn eval_index(
    index: &syn::ExprIndex,
    env: &mut crate::Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Evaluate the base expression
    let base = index.expr.eval(env, ctx)?;

    // Evaluate the index expression
    let index_val = index.index.eval(env, ctx)?;

    match base {
        // Vec indexing
        Value::Vec(vec) => {
            let idx = index_val.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!(
                    "vec index must be integer, got {}",
                    crate::error::type_name(&index_val)
                ),
                span: None,
            })?;

            vec.get(idx)
                .cloned()
                .ok_or_else(|| EvalError::IndexOutOfBounds {
                    index: idx,
                    len: vec.len(),
                    span: None,
                })
        }

        // Array indexing
        Value::Array(arr) => {
            let idx = index_val.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!(
                    "array index must be integer, got {}",
                    crate::error::type_name(&index_val)
                ),
                span: None,
            })?;

            arr.get(idx)
                .cloned()
                .ok_or_else(|| EvalError::IndexOutOfBounds {
                    index: idx,
                    len: arr.len(),
                    span: None,
                })
        }

        // String indexing (returns char)
        Value::String(s) => {
            let idx = index_val.as_usize().ok_or_else(|| EvalError::TypeError {
                message: format!(
                    "string index must be integer, got {}",
                    crate::error::type_name(&index_val)
                ),
                span: None,
            })?;

            s.chars()
                .nth(idx)
                .map(Value::Char)
                .ok_or_else(|| EvalError::IndexOutOfBounds {
                    index: idx,
                    len: s.chars().count(),
                    span: None,
                })
        }

        // HashMap indexing
        Value::HashMap(map) => {
            // Check if value is hashable
            if !crate::value::HashableValue::is_hashable(&index_val) {
                return Err(EvalError::TypeError {
                    message: format!(
                        "hashmap key must be hashable, got {}",
                        crate::error::type_name(&index_val)
                    ),
                    span: None,
                });
            }

            // Wrap in HashableValue for lookup
            let key = crate::value::HashableValue(index_val.clone());

            map.get(&key)
                .cloned()
                .ok_or_else(|| EvalError::KeyNotFound {
                    key: format!("{:?}", key),
                    span: None,
                })
        }

        // Tuple indexing is handled by ExprField, not ExprIndex
        _ => Err(EvalError::TypeError {
            message: format!("cannot index into {}", crate::error::type_name(&base)),
            span: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;

    #[test]
    fn test_vec_index() {
        let expr: syn::Expr = syn::parse_str("v[1]").unwrap();
        if let syn::Expr::Index(index) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            // Define a vec
            env.define(
                "v".to_string(),
                Value::vec(vec![Value::I64(10), Value::I64(20), Value::I64(30)]),
            );

            let result = eval_index(&index, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::I64(20));
        } else {
            panic!("Expected Index");
        }
    }

    #[test]
    fn test_array_index() {
        let expr: syn::Expr = syn::parse_str("a[2]").unwrap();
        if let syn::Expr::Index(index) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            // Define an array
            env.define(
                "a".to_string(),
                Value::array(vec![Value::I64(100), Value::I64(200), Value::I64(300)]),
            );

            let result = eval_index(&index, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::I64(300));
        } else {
            panic!("Expected Index");
        }
    }

    #[test]
    fn test_string_index() {
        let expr: syn::Expr = syn::parse_str("s[0]").unwrap();
        if let syn::Expr::Index(index) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define("s".to_string(), Value::string("hello"));

            let result = eval_index(&index, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::Char('h'));
        } else {
            panic!("Expected Index");
        }
    }

    #[test]
    fn test_hashmap_index() {
        use std::collections::HashMap;

        let expr: syn::Expr = syn::parse_str("m[key]").unwrap();
        if let syn::Expr::Index(index) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let mut map = HashMap::new();
            map.insert(
                crate::value::HashableValue(Value::string("key")),
                Value::I64(42),
            );
            env.define("m".to_string(), Value::HashMap(std::sync::Arc::new(map)));
            env.define("key".to_string(), Value::string("key"));

            let result = eval_index(&index, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::I64(42));
        } else {
            panic!("Expected Index");
        }
    }

    #[test]
    fn test_vec_index_out_of_bounds() {
        let expr: syn::Expr = syn::parse_str("v[10]").unwrap();
        if let syn::Expr::Index(index) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define(
                "v".to_string(),
                Value::vec(vec![Value::I64(1), Value::I64(2)]),
            );

            let result = eval_index(&index, &mut env, &ctx);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                EvalError::IndexOutOfBounds { .. }
            ));
        } else {
            panic!("Expected Index");
        }
    }

    #[test]
    fn test_hashmap_key_not_found() {
        use std::collections::HashMap;

        let expr: syn::Expr = syn::parse_str("m[missing]").unwrap();
        if let syn::Expr::Index(index) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let map: HashMap<crate::value::HashableValue, Value> = HashMap::new();
            env.define("m".to_string(), Value::HashMap(std::sync::Arc::new(map)));
            env.define("missing".to_string(), Value::string("missing"));

            let result = eval_index(&index, &mut env, &ctx);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), EvalError::KeyNotFound { .. }));
        } else {
            panic!("Expected Index");
        }
    }

    #[test]
    fn test_index_non_integer() {
        let expr: syn::Expr = syn::parse_str("v[x]").unwrap();
        if let syn::Expr::Index(index) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define("v".to_string(), Value::vec(vec![Value::I64(1)]));
            env.define("x".to_string(), Value::string("not_an_int"));

            let result = eval_index(&index, &mut env, &ctx);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), EvalError::TypeError { .. }));
        } else {
            panic!("Expected Index");
        }
    }
}
