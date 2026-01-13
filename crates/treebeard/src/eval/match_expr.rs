//! Match expression evaluation

use super::pattern::{apply_bindings, match_pattern};
use super::Evaluate;
use crate::{Environment, EvalContext, EvalError, Value};

impl Evaluate for syn::ExprMatch {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate the scrutinee
        let scrutinee = self.expr.eval(env, ctx)?;

        // Try each arm
        for arm in &self.arms {
            // Check if pattern matches
            if let Some(bindings) = match_pattern(&arm.pat, &scrutinee, None)? {
                // Check guard if present
                let guard_passes = if let Some((_, guard)) = &arm.guard {
                    // Temporarily add bindings for guard evaluation
                    env.push_frame();
                    apply_bindings(env, bindings.clone());
                    let guard_result = guard.eval(env, ctx);
                    env.pop_frame();

                    match guard_result? {
                        Value::Bool(b) => b,
                        other => {
                            return Err(EvalError::TypeError {
                                message: format!(
                                    "expected `bool` in match guard, found `{}`",
                                    crate::error::type_name(&other)
                                ),
                                span: None,
                            });
                        }
                    }
                } else {
                    true
                };

                if guard_passes {
                    // Pattern matches and guard passes - evaluate body
                    env.push_frame();
                    apply_bindings(env, bindings);
                    let result = arm.body.eval(env, ctx);
                    env.pop_frame();
                    return result;
                }
            }
        }

        // No arm matched
        Err(EvalError::NonExhaustiveMatch {
            value: format!("{:?}", scrutinee),
            span: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_literal_first_arm() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 1 {
                1 => 100,
                2 => 200,
                _ => 0,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx).unwrap();

        assert_eq!(result, Value::I64(100));
    }

    #[test]
    fn test_match_literal_second_arm() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 2 {
                1 => 100,
                2 => 200,
                _ => 0,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx).unwrap();

        assert_eq!(result, Value::I64(200));
    }

    #[test]
    fn test_match_wildcard() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 99 {
                1 => 100,
                2 => 200,
                _ => 0,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx).unwrap();

        assert_eq!(result, Value::I64(0));
    }

    #[test]
    fn test_match_with_guard_true() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 5 {
                x if x > 3 => 100,
                _ => 0,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx).unwrap();

        assert_eq!(result, Value::I64(100));
    }

    #[test]
    fn test_match_with_guard_false() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 2 {
                x if x > 3 => 100,
                _ => 0,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx).unwrap();

        assert_eq!(result, Value::I64(0));
    }

    #[test]
    fn test_match_guard_non_bool() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 5 {
                x if x => 100,
                _ => 0,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx);

        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::TypeError { message, .. } => {
                assert!(message.contains("expected `bool` in match guard"));
            }
            _ => panic!("Expected TypeError"),
        }
    }

    #[test]
    fn test_match_non_exhaustive() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 3 {
                1 => 100,
                2 => 200,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx);

        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::NonExhaustiveMatch { .. } => {}
            _ => panic!("Expected NonExhaustiveMatch"),
        }
    }

    #[test]
    fn test_match_variable_binding() {
        let expr: syn::ExprMatch = syn::parse_quote! {
            match 42 {
                x => x + 1,
            }
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = expr.eval(&mut env, &ctx).unwrap();

        assert_eq!(result, Value::I64(43));
    }
}
