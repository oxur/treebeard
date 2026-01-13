//! Unary operation evaluation

use crate::error::type_name;
use crate::{Environment, EvalContext, EvalError, Value};

use super::Evaluate;
use syn::spanned::Spanned;

impl Evaluate for syn::ExprUnary {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        let operand = self.expr.eval(env, ctx)?;
        let span = Some(self.op.span());

        match &self.op {
            syn::UnOp::Neg(_) => eval_neg(operand, span),
            syn::UnOp::Not(_) => eval_not(operand, span),
            syn::UnOp::Deref(_) => eval_deref(operand, span),
            _ => Err(EvalError::UnsupportedExpr {
                kind: "unknown unary operator".to_string(),
                span,
            }),
        }
    }
}

/// Evaluate unary negation (`-x`).
pub(crate) fn eval_neg(
    operand: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match operand {
        // Signed integers
        Value::I8(n) => n
            .checked_neg()
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I16(n) => n
            .checked_neg()
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I32(n) => n
            .checked_neg()
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I64(n) => n
            .checked_neg()
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I128(n) => n
            .checked_neg()
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::Isize(n) => n
            .checked_neg()
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),

        // Floats (no overflow for negation)
        Value::F32(n) => Ok(Value::F32(-n)),
        Value::F64(n) => Ok(Value::F64(-n)),

        // Unsigned integers can't be negated
        other => Err(EvalError::InvalidUnaryOperand {
            op: "-".to_string(),
            operand_type: type_name(&other).to_string(),
            span,
        }),
    }
}

/// Evaluate logical/bitwise NOT (`!x`).
fn eval_not(operand: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    match operand {
        // Logical NOT for booleans
        Value::Bool(b) => Ok(Value::Bool(!b)),

        // Bitwise NOT for integers
        Value::I8(n) => Ok(Value::I8(!n)),
        Value::I16(n) => Ok(Value::I16(!n)),
        Value::I32(n) => Ok(Value::I32(!n)),
        Value::I64(n) => Ok(Value::I64(!n)),
        Value::I128(n) => Ok(Value::I128(!n)),
        Value::Isize(n) => Ok(Value::Isize(!n)),
        Value::U8(n) => Ok(Value::U8(!n)),
        Value::U16(n) => Ok(Value::U16(!n)),
        Value::U32(n) => Ok(Value::U32(!n)),
        Value::U64(n) => Ok(Value::U64(!n)),
        Value::U128(n) => Ok(Value::U128(!n)),
        Value::Usize(n) => Ok(Value::Usize(!n)),

        other => Err(EvalError::InvalidUnaryOperand {
            op: "!".to_string(),
            operand_type: type_name(&other).to_string(),
            span,
        }),
    }
}

/// Evaluate dereference (`*x`).
fn eval_deref(operand: Value, span: Option<proc_macro2::Span>) -> Result<Value, EvalError> {
    match operand {
        Value::Ref(r) => Ok((*r.value).clone()),
        Value::RefMut(r) => {
            let guard = r.value.read().map_err(|_| EvalError::TypeError {
                message: "failed to acquire read lock on RefMut".to_string(),
                span,
            })?;
            Ok(guard.clone())
        }
        other => Err(EvalError::InvalidUnaryOperand {
            op: "*".to_string(),
            operand_type: type_name(&other).to_string(),
            span,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neg_i64() {
        let result = eval_neg(Value::I64(42), None).unwrap();
        assert_eq!(result, Value::I64(-42));
    }

    #[test]
    fn test_neg_i32() {
        let result = eval_neg(Value::I32(42), None).unwrap();
        assert_eq!(result, Value::I32(-42));
    }

    #[test]
    fn test_neg_f64() {
        let result = eval_neg(Value::F64(3.14), None).unwrap();
        assert_eq!(result, Value::F64(-3.14));
    }

    #[test]
    fn test_neg_f32() {
        let result = eval_neg(Value::F32(3.14), None).unwrap();
        assert_eq!(result, Value::F32(-3.14));
    }

    #[test]
    fn test_neg_unsigned_fails() {
        let result = eval_neg(Value::U32(42), None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::InvalidUnaryOperand { .. }
        ));
    }

    #[test]
    fn test_not_bool() {
        let result = eval_not(Value::Bool(true), None).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_not_i32() {
        let result = eval_not(Value::I32(0b1010), None).unwrap();
        assert_eq!(result, Value::I32(!0b1010));
    }

    #[test]
    fn test_not_u8() {
        let result = eval_not(Value::U8(0b1010), None).unwrap();
        assert_eq!(result, Value::U8(!0b1010));
    }

    #[test]
    fn test_not_string_fails() {
        let result = eval_not(Value::string("test"), None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::InvalidUnaryOperand { .. }
        ));
    }

    #[test]
    fn test_deref_ref() {
        use crate::value::ValueRef;
        use std::sync::Arc;
        let val = Value::I64(42);
        let ref_val = Value::Ref(ValueRef {
            value: Arc::new(val),
            tag: 0,
        });
        let result = eval_deref(ref_val, None).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_deref_non_ref_fails() {
        let result = eval_deref(Value::I64(42), None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::InvalidUnaryOperand { .. }
        ));
    }
}
