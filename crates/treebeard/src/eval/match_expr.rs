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
