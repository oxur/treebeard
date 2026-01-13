//! Struct literal evaluation

use indexmap::IndexMap;

use crate::value::StructValue;
use crate::{EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate a struct literal expression.
///
/// # Examples
///
/// - `Point { x: 1, y: 2 }` → Struct with two fields
/// - `Point { x: 10, ..old }` → Struct with update syntax
///
/// # Errors
///
/// Returns errors from evaluating field values.
/// Returns `TypeError` if the base in update syntax is not a struct.
pub fn eval_struct(
    struct_expr: &syn::ExprStruct,
    env: &mut crate::Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Get the struct type name from the path
    let type_name = path_to_string(&struct_expr.path);

    // Start with fields from base struct if using update syntax
    let mut fields: IndexMap<String, Value> = if let Some(base) = &struct_expr.rest {
        let base_val = base.eval(env, ctx)?;
        match base_val {
            Value::Struct(s) => {
                // Clone all fields from base struct
                s.fields.clone()
            }
            _ => {
                return Err(EvalError::TypeError {
                    message: format!(
                        "struct update base must be struct, got {}",
                        crate::error::type_name(&base_val)
                    ),
                    span: None,
                })
            }
        }
    } else {
        IndexMap::new()
    };

    // Evaluate and insert explicitly specified fields
    for field in &struct_expr.fields {
        let field_name = match &field.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };
        let field_value = field.expr.eval(env, ctx)?;
        fields.insert(field_name, field_value);
    }

    Ok(Value::structure(StructValue {
        type_name,
        fields,
        is_tuple_struct: false,
    }))
}

/// Convert a path to a string type name.
fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;

    #[test]
    fn test_struct_literal_simple() {
        let expr: syn::Expr = syn::parse_str("Point { x: 1, y: 2 }").unwrap();
        if let syn::Expr::Struct(struct_expr) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_struct(&struct_expr, &mut env, &ctx).unwrap();

            if let Value::Struct(s) = result {
                assert_eq!(s.type_name, "Point");
                assert_eq!(s.fields.get("x"), Some(&Value::I64(1)));
                assert_eq!(s.fields.get("y"), Some(&Value::I64(2)));
            } else {
                panic!("Expected Struct value");
            }
        } else {
            panic!("Expected Struct");
        }
    }

    #[test]
    fn test_struct_literal_with_expressions() {
        let expr: syn::Expr = syn::parse_str("Point { x: 1 + 2, y: 3 * 4 }").unwrap();
        if let syn::Expr::Struct(struct_expr) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_struct(&struct_expr, &mut env, &ctx).unwrap();

            if let Value::Struct(s) = result {
                assert_eq!(s.type_name, "Point");
                assert_eq!(s.fields.get("x"), Some(&Value::I64(3)));
                assert_eq!(s.fields.get("y"), Some(&Value::I64(12)));
            } else {
                panic!("Expected Struct value");
            }
        } else {
            panic!("Expected Struct");
        }
    }

    #[test]
    fn test_struct_literal_with_variables() {
        let expr: syn::Expr = syn::parse_str("Point { x: a, y: b }").unwrap();
        if let syn::Expr::Struct(struct_expr) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define("a".to_string(), Value::I64(10));
            env.define("b".to_string(), Value::I64(20));

            let result = eval_struct(&struct_expr, &mut env, &ctx).unwrap();

            if let Value::Struct(s) = result {
                assert_eq!(s.type_name, "Point");
                assert_eq!(s.fields.get("x"), Some(&Value::I64(10)));
                assert_eq!(s.fields.get("y"), Some(&Value::I64(20)));
            } else {
                panic!("Expected Struct value");
            }
        } else {
            panic!("Expected Struct");
        }
    }

    #[test]
    fn test_struct_literal_update_syntax() {
        let expr: syn::Expr = syn::parse_str("Point { x: 100, ..base }").unwrap();
        if let syn::Expr::Struct(struct_expr) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            // Create a base struct
            let mut base_fields = IndexMap::new();
            base_fields.insert("x".to_string(), Value::I64(1));
            base_fields.insert("y".to_string(), Value::I64(2));
            base_fields.insert("z".to_string(), Value::I64(3));

            env.define(
                "base".to_string(),
                Value::structure(StructValue {
                    type_name: "Point".to_string(),
                    fields: base_fields,
                    is_tuple_struct: false,
                }),
            );

            let result = eval_struct(&struct_expr, &mut env, &ctx).unwrap();

            if let Value::Struct(s) = result {
                assert_eq!(s.type_name, "Point");
                // x should be overridden
                assert_eq!(s.fields.get("x"), Some(&Value::I64(100)));
                // y and z should come from base
                assert_eq!(s.fields.get("y"), Some(&Value::I64(2)));
                assert_eq!(s.fields.get("z"), Some(&Value::I64(3)));
            } else {
                panic!("Expected Struct value");
            }
        } else {
            panic!("Expected Struct");
        }
    }

    #[test]
    fn test_struct_literal_empty() {
        let expr: syn::Expr = syn::parse_str("Empty {}").unwrap();
        if let syn::Expr::Struct(struct_expr) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_struct(&struct_expr, &mut env, &ctx).unwrap();

            if let Value::Struct(s) = result {
                assert_eq!(s.type_name, "Empty");
                assert!(s.fields.is_empty());
            } else {
                panic!("Expected Struct value");
            }
        } else {
            panic!("Expected Struct");
        }
    }

    #[test]
    fn test_struct_literal_qualified_path() {
        let expr: syn::Expr = syn::parse_str("module::Point { x: 1, y: 2 }").unwrap();
        if let syn::Expr::Struct(struct_expr) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let result = eval_struct(&struct_expr, &mut env, &ctx).unwrap();

            if let Value::Struct(s) = result {
                assert_eq!(s.type_name, "module::Point");
                assert_eq!(s.fields.get("x"), Some(&Value::I64(1)));
                assert_eq!(s.fields.get("y"), Some(&Value::I64(2)));
            } else {
                panic!("Expected Struct value");
            }
        } else {
            panic!("Expected Struct");
        }
    }
}
