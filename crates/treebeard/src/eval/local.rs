//! Local binding (let statement) evaluation

use crate::eval::pattern::match_pattern;
use crate::{BindingMode, Environment, EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate a local (let) binding.
///
/// # Errors
///
/// Returns `RefutablePattern` if the pattern doesn't match the value.
pub fn eval_local(
    local: &syn::Local,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<(), EvalError> {
    // Get the initializer value (or Unit if none)
    let value = if let Some(init) = &local.init {
        init.expr.eval(env, ctx)?
    } else {
        Value::Unit
    };

    // Check if mutable
    let is_mutable = is_pattern_mutable(&local.pat);

    // Match the pattern and bind
    if let Some(bindings) = match_pattern(&local.pat, &value, None)? {
        for (name, val, pat_mut) in bindings {
            let mode = if is_mutable || pat_mut {
                BindingMode::Mutable
            } else {
                BindingMode::Immutable
            };
            env.define_with_mode(name, val, mode);
        }
        Ok(())
    } else {
        Err(EvalError::RefutablePattern {
            pattern: format!("{:?}", local.pat),
            span: None,
        })
    }
}

/// Check if a pattern has the `mut` keyword.
fn is_pattern_mutable(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(pat_ident) => pat_ident.mutability.is_some(),
        syn::Pat::Type(pat_type) => is_pattern_mutable(&pat_type.pat),
        syn::Pat::Reference(pat_ref) => is_pattern_mutable(&pat_ref.pat),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_let_simple_binding() {
        let stmt: syn::Stmt = syn::parse_str("let x = 42;").unwrap();
        if let syn::Stmt::Local(local) = stmt {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            eval_local(&local, &mut env, &ctx).unwrap();
            assert_eq!(env.get("x"), Some(&Value::I64(42)));
        } else {
            panic!("Expected Local");
        }
    }

    #[test]
    fn test_let_mutable_binding() {
        let stmt: syn::Stmt = syn::parse_str("let mut x = 42;").unwrap();
        if let syn::Stmt::Local(local) = stmt {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            eval_local(&local, &mut env, &ctx).unwrap();

            // Verify it's mutable by trying to reassign
            env.assign("x", Value::I64(100)).unwrap();
            assert_eq!(env.get("x"), Some(&Value::I64(100)));
        } else {
            panic!("Expected Local");
        }
    }

    #[test]
    fn test_let_without_init() {
        let stmt: syn::Stmt = syn::parse_str("let x;").unwrap();
        if let syn::Stmt::Local(local) = stmt {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            eval_local(&local, &mut env, &ctx).unwrap();
            assert_eq!(env.get("x"), Some(&Value::Unit));
        } else {
            panic!("Expected Local");
        }
    }

    #[test]
    fn test_let_wildcard_pattern() {
        let stmt: syn::Stmt = syn::parse_str("let _ = 42;").unwrap();
        if let syn::Stmt::Local(local) = stmt {
            let mut env = Environment::new();
            let ctx = EvalContext::default();

            eval_local(&local, &mut env, &ctx).unwrap();
            // Wildcard doesn't bind anything
            assert_eq!(env.get("_"), None);
        } else {
            panic!("Expected Local");
        }
    }
}
