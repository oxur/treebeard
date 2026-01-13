//! Field expression evaluation

use crate::{EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate a field access expression.
///
/// Supports field access on structs, tuples, and enum variants.
///
/// # Errors
///
/// Returns `UndefinedField` if the field doesn't exist.
/// Returns `IndexOutOfBounds` for tuple field out of range.
/// Returns `TypeError` if the base value doesn't support field access.
pub fn eval_field(
    field: &syn::ExprField,
    env: &mut crate::Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Evaluate the base expression
    let base = field.base.eval(env, ctx)?;

    match &field.member {
        // Named field access (struct)
        syn::Member::Named(ident) => {
            let field_name = ident.to_string();

            match base {
                Value::Struct(s) => {
                    s.fields
                        .get(&field_name)
                        .cloned()
                        .ok_or_else(|| EvalError::UndefinedField {
                            field: field_name,
                            type_name: s.type_name.clone(),
                            span: None,
                        })
                }

                Value::Enum(e) => {
                    // For enum variants with struct data
                    match &e.data {
                        crate::value::EnumData::Struct(fields) => fields
                            .get(&field_name)
                            .cloned()
                            .ok_or_else(|| EvalError::UndefinedField {
                                field: field_name,
                                type_name: format!("{}::{}", e.type_name, e.variant),
                                span: None,
                            }),
                        _ => Err(EvalError::TypeError {
                            message: format!(
                                "enum variant {}::{} doesn't have named fields",
                                e.type_name, e.variant
                            ),
                            span: None,
                        }),
                    }
                }

                _ => Err(EvalError::TypeError {
                    message: format!(
                        "cannot access field `{}` on {}",
                        field_name,
                        crate::error::type_name(&base)
                    ),
                    span: None,
                }),
            }
        }

        // Unnamed field access (tuple)
        syn::Member::Unnamed(index) => {
            let idx = index.index as usize;

            match base {
                Value::Tuple(t) => t
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| EvalError::IndexOutOfBounds {
                        index: idx,
                        len: t.len(),
                        span: None,
                    }),

                Value::Enum(e) => {
                    // For enum variants with tuple data
                    match &e.data {
                        crate::value::EnumData::Tuple(fields) => fields
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| EvalError::IndexOutOfBounds {
                                index: idx,
                                len: fields.len(),
                                span: None,
                            }),
                        _ => Err(EvalError::TypeError {
                            message: format!(
                                "enum variant {}::{} doesn't have tuple fields",
                                e.type_name, e.variant
                            ),
                            span: None,
                        }),
                    }
                }

                _ => Err(EvalError::TypeError {
                    message: format!(
                        "cannot access field {} on {}",
                        idx,
                        crate::error::type_name(&base)
                    ),
                    span: None,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{EnumData, EnumValue, StructValue};
    use crate::Environment;
    use indexmap::IndexMap;

    #[test]
    fn test_struct_field_access() {
        let expr: syn::Expr = syn::parse_str("p.x").unwrap();
        if let syn::Expr::Field(field) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let mut fields = IndexMap::new();
            fields.insert("x".to_string(), Value::I64(10));
            fields.insert("y".to_string(), Value::I64(20));

            env.define(
                "p".to_string(),
                Value::structure(StructValue {
                    type_name: "Point".to_string(),
                    fields,
                    is_tuple_struct: false,
                }),
            );

            let result = eval_field(&field, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::I64(10));
        } else {
            panic!("Expected Field");
        }
    }

    #[test]
    fn test_tuple_field_access() {
        let expr: syn::Expr = syn::parse_str("t.0").unwrap();
        if let syn::Expr::Field(field) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define(
                "t".to_string(),
                Value::tuple(vec![Value::I64(42), Value::string("hello")]),
            );

            let result = eval_field(&field, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::I64(42));
        } else {
            panic!("Expected Field");
        }
    }

    #[test]
    fn test_tuple_field_second_element() {
        let expr: syn::Expr = syn::parse_str("t.1").unwrap();
        if let syn::Expr::Field(field) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define(
                "t".to_string(),
                Value::tuple(vec![Value::I64(42), Value::string("hello")]),
            );

            let result = eval_field(&field, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::string("hello"));
        } else {
            panic!("Expected Field");
        }
    }

    #[test]
    fn test_enum_struct_variant_field() {
        let expr: syn::Expr = syn::parse_str("e.x").unwrap();
        if let syn::Expr::Field(field) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let mut fields = IndexMap::new();
            fields.insert("x".to_string(), Value::I64(100));

            env.define(
                "e".to_string(),
                Value::enumeration(EnumValue {
                    type_name: "MyEnum".to_string(),
                    variant: "StructVariant".to_string(),
                    data: EnumData::Struct(fields),
                }),
            );

            let result = eval_field(&field, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::I64(100));
        } else {
            panic!("Expected Field");
        }
    }

    #[test]
    fn test_enum_tuple_variant_field() {
        let expr: syn::Expr = syn::parse_str("e.0").unwrap();
        if let syn::Expr::Field(field) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define(
                "e".to_string(),
                Value::enumeration(EnumValue {
                    type_name: "MyEnum".to_string(),
                    variant: "TupleVariant".to_string(),
                    data: EnumData::Tuple(vec![Value::I64(42), Value::string("test")]),
                }),
            );

            let result = eval_field(&field, &mut env, &ctx).unwrap();
            assert_eq!(result, Value::I64(42));
        } else {
            panic!("Expected Field");
        }
    }

    #[test]
    fn test_struct_undefined_field() {
        let expr: syn::Expr = syn::parse_str("p.z").unwrap();
        if let syn::Expr::Field(field) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            let mut fields = IndexMap::new();
            fields.insert("x".to_string(), Value::I64(10));

            env.define(
                "p".to_string(),
                Value::structure(StructValue {
                    type_name: "Point".to_string(),
                    fields,
                    is_tuple_struct: false,
                }),
            );

            let result = eval_field(&field, &mut env, &ctx);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                EvalError::UndefinedField { .. }
            ));
        } else {
            panic!("Expected Field");
        }
    }

    #[test]
    fn test_tuple_field_out_of_bounds() {
        let expr: syn::Expr = syn::parse_str("t.5").unwrap();
        if let syn::Expr::Field(field) = expr {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            env.define("t".to_string(), Value::tuple(vec![Value::I64(1)]));

            let result = eval_field(&field, &mut env, &ctx);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                EvalError::IndexOutOfBounds { .. }
            ));
        } else {
            panic!("Expected Field");
        }
    }
}
