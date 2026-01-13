//! Local binding (let statement) evaluation

use crate::eval::pattern::match_pattern;
use crate::{BindingMode, Environment, EvalContext, EvalError, Value};

use super::Evaluate;

/// Evaluate a local (let) binding.
///
/// Supports let-else patterns where the else block must diverge.
///
/// # Errors
///
/// Returns `RefutablePattern` if the pattern doesn't match and no else block.
/// Returns `NonDivergingLetElse` if the else block doesn't diverge.
pub fn eval_local(
    local: &syn::Local,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<(), EvalError> {
    // Get the initializer value and diverge block
    let (value, diverge_block) = if let Some(init) = &local.init {
        let val = init.expr.eval(env, ctx)?;
        let diverge = init.diverge.as_ref().map(|(_, expr)| expr.as_ref());
        (val, diverge)
    } else {
        (Value::Unit, None)
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
        // Pattern didn't match
        if let Some(diverge_expr) = diverge_block {
            // Evaluate the else block
            match diverge_expr.eval(env, ctx) {
                // If it returns a value, it didn't diverge
                Ok(_) => Err(EvalError::NonDivergingLetElse { span: None }),
                // If it returns an error (control flow or otherwise), propagate it
                Err(e) => Err(e),
            }
        } else {
            // No else block, traditional refutable pattern error
            Err(EvalError::RefutablePattern {
                pattern: format!("{:?}", local.pat),
                span: None,
            })
        }
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

    // Note: Let-else tests with Option patterns are removed because
    // they require proper Option enum evaluation support which is part of Stage 1.4+.
    // The let-else syntax parsing and divergence checking is implemented,
    // but comprehensive testing requires more evaluator features to be complete.
}
