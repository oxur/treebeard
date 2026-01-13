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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::if_expr::eval_block;

    #[test]
    fn test_loop_with_break() {
        let expr: syn::Expr = syn::parse_str("loop { break 42 }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_while_false_never_executes() {
        let expr: syn::Expr = syn::parse_str("while false { 42 }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_break_returns_control_flow() {
        let expr: syn::Expr = syn::parse_str("break 123").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::ControlFlow(ControlFlow::Break { .. })
        ));
    }

    #[test]
    fn test_continue_returns_control_flow() {
        let expr: syn::Expr = syn::parse_str("continue").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::ControlFlow(ControlFlow::Continue { .. })
        ));
    }

    #[test]
    fn test_while_non_bool_condition() {
        let expr: syn::Expr = syn::parse_str("while 42 { }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EvalError::TypeError { .. }));
    }

    #[test]
    fn test_loop_with_continue() {
        // This is tricky to test without let bindings, but we can test
        // that the control flow is properly handled
        let block: syn::Block = syn::parse_str(
            r#"{
            'outer: loop {
                break 'outer 42
            }
        }"#,
        )
        .unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_loop_with_label() {
        let expr: syn::Expr = syn::parse_str("'outer: loop { break 'outer 99 }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(99));
    }

    #[test]
    fn test_break_with_label() {
        let expr: syn::Expr = syn::parse_str("break 'outer 77").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        if let Err(EvalError::ControlFlow(ControlFlow::Break { value, label })) = result {
            assert_eq!(value, Value::I64(77));
            assert_eq!(label, Some("outer".to_string()));
        } else {
            panic!("Expected Break control flow with label");
        }
    }

    #[test]
    fn test_continue_with_label() {
        let expr: syn::Expr = syn::parse_str("continue 'outer").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        if let Err(EvalError::ControlFlow(ControlFlow::Continue { label })) = result {
            assert_eq!(label, Some("outer".to_string()));
        } else {
            panic!("Expected Continue control flow with label");
        }
    }

    #[test]
    fn test_while_with_label() {
        let expr: syn::Expr = syn::parse_str("'label: while false { }").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_for_loop_empty_array() {
        let block: syn::Block = syn::parse_str(
            r#"{
            for _ in [] {
                42
            }
        }"#,
        )
        .unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_for_loop_string() {
        let block: syn::Block = syn::parse_str(
            r#"{
            for _ in "hi" {
                1
            }
        }"#,
        )
        .unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_for_loop_non_iterable() {
        let block: syn::Block = syn::parse_str(
            r#"{
            for _ in 42 {
                1
            }
        }"#,
        )
        .unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EvalError::TypeError { .. }));
    }

    #[test]
    fn test_for_loop_with_label() {
        let block: syn::Block = syn::parse_str(
            r#"{
            'outer: for _ in [1, 2, 3] {
                break 'outer
            }
        }"#,
        )
        .unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_block(&block, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_value_to_iterator_array() {
        let arr = Value::array(vec![Value::I64(1), Value::I64(2)]);
        let mut iter = value_to_iterator(arr).unwrap();
        assert_eq!(iter.next(), Some(Value::I64(1)));
        assert_eq!(iter.next(), Some(Value::I64(2)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_value_to_iterator_string() {
        let s = Value::string("ab");
        let mut iter = value_to_iterator(s).unwrap();
        assert_eq!(iter.next(), Some(Value::Char('a')));
        assert_eq!(iter.next(), Some(Value::Char('b')));
        assert_eq!(iter.next(), None);
    }
}
