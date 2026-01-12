//! Loop expression evaluation

use super::if_expr::eval_block;
use super::Evaluate;
use crate::eval::control::ControlFlow;
use crate::{Environment, EvalContext, EvalError, Value};

// ═══════════════════════════════════════════════════════════════════════
// loop expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprLoop {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.name.ident.to_string());

        loop {
            // Check for interruption
            if ctx.is_interrupted() {
                return Err(EvalError::Interrupted);
            }

            // Evaluate body
            match eval_block(&self.body, env, ctx) {
                Ok(_) => {
                    // Continue looping
                }
                Err(EvalError::ControlFlow(cf)) => {
                    match &cf {
                        ControlFlow::Break { value, .. } if cf.matches_label(label.as_deref()) => {
                            return Ok(value.clone());
                        }
                        ControlFlow::Continue { .. } if cf.matches_label(label.as_deref()) => {
                            // Continue to next iteration
                        }
                        _ => {
                            // Propagate (different label or return)
                            return Err(EvalError::ControlFlow(cf));
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// while expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprWhile {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.name.ident.to_string());

        loop {
            // Check for interruption
            if ctx.is_interrupted() {
                return Err(EvalError::Interrupted);
            }

            // Evaluate condition
            let cond = self.cond.eval(env, ctx)?;
            let cond_bool = match cond {
                Value::Bool(b) => b,
                other => {
                    return Err(EvalError::TypeError {
                        message: format!(
                            "expected `bool` in while condition, found `{}`",
                            crate::error::type_name(&other)
                        ),
                        span: None,
                    });
                }
            };

            if !cond_bool {
                // Condition false, exit loop
                return Ok(Value::Unit);
            }

            // Evaluate body
            match eval_block(&self.body, env, ctx) {
                Ok(_) => {
                    // Continue looping
                }
                Err(EvalError::ControlFlow(cf)) => {
                    match &cf {
                        ControlFlow::Break { .. } if cf.matches_label(label.as_deref()) => {
                            // while loops always return unit (break value ignored)
                            return Ok(Value::Unit);
                        }
                        ControlFlow::Continue { .. } if cf.matches_label(label.as_deref()) => {
                            // Continue to next iteration
                        }
                        _ => {
                            // Propagate
                            return Err(EvalError::ControlFlow(cf));
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// for expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprForLoop {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.name.ident.to_string());

        // Evaluate the iterator expression
        let iter_value = self.expr.eval(env, ctx)?;

        // Convert to an iterator
        let iterator = value_to_iterator(iter_value)?;

        for item in iterator {
            // Check for interruption
            if ctx.is_interrupted() {
                return Err(EvalError::Interrupted);
            }

            // Push frame for loop body
            env.push_frame();

            // Bind the pattern
            if let Some(bindings) = super::pattern::match_pattern(&self.pat, &item, None)? {
                super::pattern::apply_bindings(env, bindings);
            } else {
                env.pop_frame();
                return Err(EvalError::RefutablePattern {
                    pattern: format!("{:?}", self.pat),
                    span: None,
                });
            }

            // Evaluate body
            let result = eval_block(&self.body, env, ctx);

            env.pop_frame();

            match result {
                Ok(_) => {
                    // Continue looping
                }
                Err(EvalError::ControlFlow(cf)) => {
                    match &cf {
                        ControlFlow::Break { .. } if cf.matches_label(label.as_deref()) => {
                            return Ok(Value::Unit);
                        }
                        ControlFlow::Continue { .. } if cf.matches_label(label.as_deref()) => {
                            // Continue to next iteration
                        }
                        _ => {
                            return Err(EvalError::ControlFlow(cf));
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(Value::Unit)
    }
}

/// Convert a Value to an iterator of Values.
fn value_to_iterator(value: Value) -> Result<Box<dyn Iterator<Item = Value>>, EvalError> {
    match value {
        Value::Vec(elements) => Ok(Box::new(
            elements.iter().cloned().collect::<Vec<_>>().into_iter(),
        )),
        Value::Array(elements) => Ok(Box::new(
            elements.iter().cloned().collect::<Vec<_>>().into_iter(),
        )),
        Value::String(s) => Ok(Box::new(
            s.chars().map(Value::Char).collect::<Vec<_>>().into_iter(),
        )),
        // Range values would go here if we had them
        other => Err(EvalError::TypeError {
            message: format!("`{}` is not an iterator", crate::error::type_name(&other)),
            span: None,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// break expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprBreak {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.ident.to_string());

        let value = if let Some(expr) = &self.expr {
            expr.eval(env, ctx)?
        } else {
            Value::Unit
        };

        Err(EvalError::ControlFlow(ControlFlow::Break { value, label }))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// continue expression
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::ExprContinue {
    fn eval(&self, _env: &mut Environment, _ctx: &EvalContext) -> Result<Value, EvalError> {
        let label = self.label.as_ref().map(|l| l.ident.to_string());

        Err(EvalError::ControlFlow(ControlFlow::Continue { label }))
    }
}
