//! Top-level item evaluation

use std::sync::Arc;

use crate::{BindingMode, Environment, EvalContext, EvalError, FunctionValue, Value};

use super::function::function_from_item;
use super::Evaluate;

/// Evaluate a top-level item.
///
/// # Errors
///
/// Returns `UnsupportedExpr` for items that aren't yet implemented.
pub fn eval_item(
    item: &syn::Item,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    match item {
        syn::Item::Fn(item_fn) => {
            let func = function_from_item(item_fn)?;
            let name = func.name.clone();
            // ALLOW: syn::Block is Send + Sync (it's just AST data),
            // but clippy can't verify this automatically
            #[allow(clippy::arc_with_non_send_sync)]
            let func_value = Value::Function(Arc::new(func));
            env.define(name, func_value);
            Ok(Value::Unit)
        }

        syn::Item::Const(item_const) => {
            // Evaluate the const expression
            let value = item_const.expr.eval(env, ctx)?;
            let name = item_const.ident.to_string();
            env.define(name, value);
            Ok(Value::Unit)
        }

        syn::Item::Static(item_static) => {
            // Evaluate the static expression
            let value = item_static.expr.eval(env, ctx)?;
            let name = item_static.ident.to_string();
            // Statics are mutable by default in the interpreter
            env.define_with_mode(
                name,
                value,
                match item_static.mutability {
                    syn::StaticMutability::Mut(_) => BindingMode::Mutable,
                    syn::StaticMutability::None => BindingMode::Immutable,
                    _ => BindingMode::Immutable,
                },
            );
            Ok(Value::Unit)
        }

        // Struct/Enum definitions - just register the type name for now
        syn::Item::Struct(item_struct) => {
            let name = item_struct.ident.to_string();
            // Store struct definition for constructor calls
            // For now, we don't need to do anything special
            // Struct literals will be handled in expressions
            let _ = name;
            Ok(Value::Unit)
        }

        syn::Item::Enum(item_enum) => {
            let name = item_enum.ident.to_string();
            let _ = name;
            Ok(Value::Unit)
        }

        // Impl blocks - register methods
        syn::Item::Impl(item_impl) => {
            // For now, just evaluate any associated functions
            for impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(method) = impl_item {
                    let func = function_from_impl_method(method, &item_impl.self_ty)?;
                    let name = func.name.clone();
                    // ALLOW: syn::Block is Send + Sync (it's just AST data),
                    // but clippy can't verify this automatically
                    #[allow(clippy::arc_with_non_send_sync)]
                    let func_value = Value::Function(Arc::new(func));
                    env.define(name, func_value);
                }
            }
            Ok(Value::Unit)
        }

        // Type aliases - no runtime effect
        syn::Item::Type(_) => Ok(Value::Unit),

        // Use statements - no runtime effect (imports are resolved at parse time)
        syn::Item::Use(_) => Ok(Value::Unit),

        // Modules - not yet supported
        syn::Item::Mod(_) => Err(EvalError::UnsupportedExpr {
            kind: "module definition".to_string(),
            span: None,
        }),

        // Traits - not yet supported
        syn::Item::Trait(_) => Err(EvalError::UnsupportedExpr {
            kind: "trait definition".to_string(),
            span: None,
        }),

        // Everything else
        _ => Err(EvalError::UnsupportedExpr {
            kind: format!("item type: {:?}", std::mem::discriminant(item)),
            span: None,
        }),
    }
}

/// Extract a FunctionValue from an impl method.
fn function_from_impl_method(
    method: &syn::ImplItemFn,
    _self_ty: &syn::Type,
) -> Result<FunctionValue, EvalError> {
    let name = method.sig.ident.to_string();
    let params = extract_method_params(&method.sig)?;
    let body = method.block.clone();

    Ok(FunctionValue::new(name, params, body))
}

/// Extract parameter names from a method signature.
fn extract_method_params(sig: &syn::Signature) -> Result<Vec<String>, EvalError> {
    let mut params = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat_type) => {
                let name = extract_pat_name(&pat_type.pat)?;
                params.push(name);
            }
            syn::FnArg::Receiver(_) => {
                params.push("self".to_string());
            }
        }
    }

    Ok(params)
}

/// Extract a name from a pattern.
fn extract_pat_name(pat: &syn::Pat) -> Result<String, EvalError> {
    match pat {
        syn::Pat::Ident(pat_ident) => Ok(pat_ident.ident.to_string()),
        syn::Pat::Wild(_) => Ok("_".to_string()),
        syn::Pat::Reference(pat_ref) => extract_pat_name(&pat_ref.pat),
        syn::Pat::Type(pat_type) => extract_pat_name(&pat_type.pat),
        _ => Err(EvalError::UnsupportedExpr {
            kind: format!("complex pattern in parameter: {:?}", pat),
            span: None,
        }),
    }
}

/// Evaluate a sequence of items (top-level forms).
///
/// # Errors
///
/// Returns errors from individual item evaluation.
pub fn eval_items(
    items: &[syn::Item],
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let mut last_value = Value::Unit;

    for item in items {
        last_value = eval_item(item, env, ctx)?;
    }

    Ok(last_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_fn_item() {
        let source = "fn test() -> i64 { 42 }";
        let item: syn::Item = syn::parse_str(source).unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_item(&item, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);

        let func = env.get("test").unwrap();
        assert!(matches!(func, Value::Function(_)));
    }

    #[test]
    fn test_eval_const_item() {
        let source = "const X: i64 = 42;";
        let item: syn::Item = syn::parse_str(source).unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_item(&item, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);

        let value = env.get("X").unwrap();
        assert_eq!(value, &Value::I64(42));
    }

    #[test]
    fn test_eval_static_item() {
        let source = "static Y: i64 = 100;";
        let item: syn::Item = syn::parse_str(source).unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_item(&item, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);

        let value = env.get("Y").unwrap();
        assert_eq!(value, &Value::I64(100));
    }

    #[test]
    fn test_eval_items_sequence() {
        let source = vec!["const A: i64 = 1;", "fn get_a() -> i64 { A }"];

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        for src in source {
            let item: syn::Item = syn::parse_str(src).unwrap();
            eval_item(&item, &mut env, &ctx).unwrap();
        }

        assert!(env.get("A").is_some());
        assert!(env.get("get_a").is_some());
    }
}
