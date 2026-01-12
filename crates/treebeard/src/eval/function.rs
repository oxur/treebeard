//! Function definition evaluation

use std::sync::Arc;

use crate::{Environment, EvalError, FunctionValue, Value};

/// Extract a FunctionValue from a syn::ItemFn.
///
/// This converts a `syn::ItemFn` AST node into a runtime `FunctionValue`
/// that can be stored in the environment and called later.
///
/// # Errors
///
/// Returns `UnsupportedExpr` for complex parameter patterns that aren't supported.
pub fn function_from_item(item_fn: &syn::ItemFn) -> Result<FunctionValue, EvalError> {
    let name = item_fn.sig.ident.to_string();

    // Extract parameter names
    let params = extract_params(&item_fn.sig)?;

    // Store the body
    let body = item_fn.block.as_ref().clone();

    Ok(FunctionValue::new(name, params, body))
}

/// Extract parameter names from a function signature.
///
/// # Errors
///
/// Returns `UnsupportedExpr` for complex parameter patterns.
fn extract_params(sig: &syn::Signature) -> Result<Vec<String>, EvalError> {
    let mut params = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat_type) => {
                // Extract the pattern (usually just an identifier)
                let name = extract_pat_name(&pat_type.pat)?;
                params.push(name);
            }
            syn::FnArg::Receiver(_) => {
                // self parameter - we'll handle this as a special case
                params.push("self".to_string());
            }
        }
    }

    Ok(params)
}

/// Extract a name from a pattern (for function parameters).
///
/// Supports simple patterns like identifiers, wildcards, and references.
///
/// # Errors
///
/// Returns `UnsupportedExpr` for complex patterns like tuples or structs.
fn extract_pat_name(pat: &syn::Pat) -> Result<String, EvalError> {
    match pat {
        syn::Pat::Ident(pat_ident) => Ok(pat_ident.ident.to_string()),
        syn::Pat::Wild(_) => Ok("_".to_string()),
        syn::Pat::Reference(pat_ref) => extract_pat_name(&pat_ref.pat),
        syn::Pat::Type(pat_type) => extract_pat_name(&pat_type.pat),
        _ => Err(EvalError::UnsupportedExpr {
            kind: format!("complex pattern in function parameter: {:?}", pat),
            span: None,
        }),
    }
}

/// Define a function in the environment.
///
/// Extracts the function from the `syn::ItemFn` and stores it in the environment
/// under the function's name.
///
/// # Errors
///
/// Returns errors from `function_from_item` if the function cannot be extracted.
pub fn define_function(item_fn: &syn::ItemFn, env: &mut Environment) -> Result<(), EvalError> {
    let func = function_from_item(item_fn)?;
    let name = func.name.clone();
    // ALLOW: syn::Block is Send + Sync (it's just AST data),
    // but clippy can't verify this automatically
    #[allow(clippy::arc_with_non_send_sync)]
    let func_value = Value::Function(Arc::new(func));
    env.define(name, func_value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_from_item_simple() {
        let source = "fn add(a: i64, b: i64) -> i64 { a + b }";
        let item_fn: syn::ItemFn = syn::parse_str(source).unwrap();

        let func = function_from_item(&item_fn).unwrap();
        assert_eq!(func.name, "add");
        assert_eq!(func.params, vec!["a", "b"]);
    }

    #[test]
    fn test_function_from_item_no_params() {
        let source = "fn get_answer() -> i64 { 42 }";
        let item_fn: syn::ItemFn = syn::parse_str(source).unwrap();

        let func = function_from_item(&item_fn).unwrap();
        assert_eq!(func.name, "get_answer");
        assert_eq!(func.params.len(), 0);
    }

    #[test]
    fn test_extract_params_with_references() {
        let source = "fn process(a: &str, b: &mut i64) -> () {}";
        let item_fn: syn::ItemFn = syn::parse_str(source).unwrap();

        let params = extract_params(&item_fn.sig).unwrap();
        assert_eq!(params, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_params_with_wildcard() {
        let source = "fn ignore(_: i64) -> () {}";
        let item_fn: syn::ItemFn = syn::parse_str(source).unwrap();

        let params = extract_params(&item_fn.sig).unwrap();
        assert_eq!(params, vec!["_"]);
    }

    #[test]
    fn test_define_function() {
        let source = "fn test() -> i64 { 42 }";
        let item_fn: syn::ItemFn = syn::parse_str(source).unwrap();

        let mut env = Environment::new();
        define_function(&item_fn, &mut env).unwrap();

        let func_val = env.get("test").unwrap();
        assert!(matches!(func_val, Value::Function(_)));
    }
}
