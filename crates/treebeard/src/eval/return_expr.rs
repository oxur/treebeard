//! Return expression evaluation

use crate::eval::control::ControlFlow;
use crate::{Environment, EvalContext, EvalError, Value};

use super::Evaluate;

impl Evaluate for syn::ExprReturn {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let value = if let Some(expr) = &self.expr {
            expr.eval(env, ctx)?
        } else {
            Value::Unit
        };

        // Return is implemented as a control flow error
        Err(EvalError::ControlFlow(ControlFlow::Return { value }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_with_value() {
        let expr: syn::Expr = syn::parse_str("return 42").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(matches!(
            result,
            Err(EvalError::ControlFlow(ControlFlow::Return { .. }))
        ));

        if let Err(EvalError::ControlFlow(ControlFlow::Return { value })) = result {
            assert_eq!(value, Value::I64(42));
        }
    }

    #[test]
    fn test_return_without_value() {
        let expr: syn::Expr = syn::parse_str("return").unwrap();
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(matches!(
            result,
            Err(EvalError::ControlFlow(ControlFlow::Return { .. }))
        ));

        if let Err(EvalError::ControlFlow(ControlFlow::Return { value })) = result {
            assert_eq!(value, Value::Unit);
        }
    }
}
