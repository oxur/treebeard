//! Binary operation evaluation

use crate::error::type_name;
use crate::{Environment, EvalContext, EvalError, Value};

use super::Evaluate;
use syn::spanned::Spanned;

impl Evaluate for syn::ExprBinary {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Short-circuit evaluation for && and ||
        match &self.op {
            syn::BinOp::And(_) => return eval_and(&self.left, &self.right, env, ctx),
            syn::BinOp::Or(_) => return eval_or(&self.left, &self.right, env, ctx),
            _ => {}
        }

        // Handle compound assignment operators by desugaring: x += y  →  x = x + y
        if is_assignment_op(&self.op) {
            return eval_compound_assignment(self, env, ctx);
        }

        // Evaluate both operands
        let left = self.left.eval(env, ctx)?;
        let right = self.right.eval(env, ctx)?;
        let span = Some(self.op.span());

        match &self.op {
            // Arithmetic
            syn::BinOp::Add(_) => eval_add(left, right, span),
            syn::BinOp::Sub(_) => eval_sub(left, right, span),
            syn::BinOp::Mul(_) => eval_mul(left, right, span),
            syn::BinOp::Div(_) => eval_div(left, right, span),
            syn::BinOp::Rem(_) => eval_rem(left, right, span),

            // Comparison
            syn::BinOp::Eq(_) => Ok(Value::Bool(left == right)),
            syn::BinOp::Ne(_) => Ok(Value::Bool(left != right)),
            syn::BinOp::Lt(_) => eval_lt(left, right, span),
            syn::BinOp::Le(_) => eval_le(left, right, span),
            syn::BinOp::Gt(_) => eval_gt(left, right, span),
            syn::BinOp::Ge(_) => eval_ge(left, right, span),

            // Bitwise
            syn::BinOp::BitAnd(_) => eval_bitand(left, right, span),
            syn::BinOp::BitOr(_) => eval_bitor(left, right, span),
            syn::BinOp::BitXor(_) => eval_bitxor(left, right, span),
            syn::BinOp::Shl(_) => eval_shl(left, right, span),
            syn::BinOp::Shr(_) => eval_shr(left, right, span),

            // Logical (already handled above with short-circuit)
            syn::BinOp::And(_) | syn::BinOp::Or(_) => unreachable!(),

            // Assignment operators already handled above
            syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_) => unreachable!(),

            _ => Err(EvalError::UnsupportedExpr {
                kind: "unknown binary operator".to_string(),
                span,
            }),
        }
    }
}

/// Check if a binary operator is a compound assignment operator.
fn is_assignment_op(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Evaluate a compound assignment expression by desugaring it.
///
/// Converts `x += y` to `x = x + y` and similar for other operators.
fn eval_compound_assignment(
    binary: &syn::ExprBinary,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Get the current value of the left side
    let left_val = binary.left.eval(env, ctx)?;

    // Evaluate the right side
    let right_val = binary.right.eval(env, ctx)?;

    let span = Some(binary.op.span());

    // Apply the underlying operation
    let new_val = match &binary.op {
        syn::BinOp::AddAssign(_) => eval_add(left_val, right_val, span)?,
        syn::BinOp::SubAssign(_) => eval_sub(left_val, right_val, span)?,
        syn::BinOp::MulAssign(_) => eval_mul(left_val, right_val, span)?,
        syn::BinOp::DivAssign(_) => eval_div(left_val, right_val, span)?,
        syn::BinOp::RemAssign(_) => eval_rem(left_val, right_val, span)?,
        syn::BinOp::BitAndAssign(_) => eval_bitand(left_val, right_val, span)?,
        syn::BinOp::BitOrAssign(_) => eval_bitor(left_val, right_val, span)?,
        syn::BinOp::BitXorAssign(_) => eval_bitxor(left_val, right_val, span)?,
        syn::BinOp::ShlAssign(_) => eval_shl(left_val, right_val, span)?,
        syn::BinOp::ShrAssign(_) => eval_shr(left_val, right_val, span)?,
        _ => unreachable!(),
    };

    // Assign the new value back
    // For simple variable assignments only (complex lvalues not yet supported)
    if let syn::Expr::Path(path) = binary.left.as_ref() {
        let name = super::path::path_to_string(&path.path);
        env.assign(&name, new_val).map_err(EvalError::from)?;
        Ok(Value::Unit)
    } else {
        Err(EvalError::InvalidAssignTarget {
            kind: "compound assignment to complex lvalue (not yet supported)".to_string(),
            span,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Short-Circuit Logical Operators
// ═══════════════════════════════════════════════════════════════════════

fn eval_and(
    left: &syn::Expr,
    right: &syn::Expr,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let left_val = left.eval(env, ctx)?;
    match left_val {
        Value::Bool(false) => Ok(Value::Bool(false)), // Short-circuit
        Value::Bool(true) => {
            let right_val = right.eval(env, ctx)?;
            match right_val {
                Value::Bool(b) => Ok(Value::Bool(b)),
                other => Err(EvalError::InvalidBinaryOperands {
                    op: "&&".to_string(),
                    left_type: "bool".to_string(),
                    right_type: type_name(&other).to_string(),
                    span: None,
                }),
            }
        }
        other => Err(EvalError::InvalidBinaryOperands {
            op: "&&".to_string(),
            left_type: type_name(&other).to_string(),
            right_type: "?".to_string(),
            span: None,
        }),
    }
}

fn eval_or(
    left: &syn::Expr,
    right: &syn::Expr,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let left_val = left.eval(env, ctx)?;
    match left_val {
        Value::Bool(true) => Ok(Value::Bool(true)), // Short-circuit
        Value::Bool(false) => {
            let right_val = right.eval(env, ctx)?;
            match right_val {
                Value::Bool(b) => Ok(Value::Bool(b)),
                other => Err(EvalError::InvalidBinaryOperands {
                    op: "||".to_string(),
                    left_type: "bool".to_string(),
                    right_type: type_name(&other).to_string(),
                    span: None,
                }),
            }
        }
        other => Err(EvalError::InvalidBinaryOperands {
            op: "||".to_string(),
            left_type: type_name(&other).to_string(),
            right_type: "?".to_string(),
            span: None,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Arithmetic Operations
// ═══════════════════════════════════════════════════════════════════════

fn eval_add(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match (&left, &right) {
        // String concatenation
        (Value::String(a), Value::String(b)) => {
            Ok(Value::string(format!("{}{}", a.as_str(), b.as_str())))
        }

        // Numeric addition
        _ => eval_add_numeric(left, right, span),
    }
}

fn eval_add_numeric(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a
            .checked_add(b)
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a
            .checked_add(b)
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a
            .checked_add(b)
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a
            .checked_add(b)
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a
            .checked_add(b)
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a
            .checked_add(b)
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a
            .checked_add(b)
            .map(Value::U8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a
            .checked_add(b)
            .map(Value::U16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a
            .checked_add(b)
            .map(Value::U32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a
            .checked_add(b)
            .map(Value::U64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a
            .checked_add(b)
            .map(Value::U128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a
            .checked_add(b)
            .map(Value::Usize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a + b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a + b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "+".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_sub(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a
            .checked_sub(b)
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a
            .checked_sub(b)
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a
            .checked_sub(b)
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a
            .checked_sub(b)
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a
            .checked_sub(b)
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a
            .checked_sub(b)
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a
            .checked_sub(b)
            .map(Value::U8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a
            .checked_sub(b)
            .map(Value::U16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a
            .checked_sub(b)
            .map(Value::U32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a
            .checked_sub(b)
            .map(Value::U64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a
            .checked_sub(b)
            .map(Value::U128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a
            .checked_sub(b)
            .map(Value::Usize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a - b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a - b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "-".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_mul(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a
            .checked_mul(b)
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a
            .checked_mul(b)
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a
            .checked_mul(b)
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a
            .checked_mul(b)
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a
            .checked_mul(b)
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a
            .checked_mul(b)
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a
            .checked_mul(b)
            .map(Value::U8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a
            .checked_mul(b)
            .map(Value::U16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a
            .checked_mul(b)
            .map(Value::U32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a
            .checked_mul(b)
            .map(Value::U64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a
            .checked_mul(b)
            .map(Value::U128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a
            .checked_mul(b)
            .map(Value::Usize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a * b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a * b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "*".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_div(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    // Check for division by zero
    let is_zero = matches!(
        &right,
        Value::I8(0)
            | Value::I16(0)
            | Value::I32(0)
            | Value::I64(0)
            | Value::I128(0)
            | Value::Isize(0)
            | Value::U8(0)
            | Value::U16(0)
            | Value::U32(0)
            | Value::U64(0)
            | Value::U128(0)
            | Value::Usize(0)
    ) || matches!(&right, Value::F32(f) if *f == 0.0)
        || matches!(&right, Value::F64(f) if *f == 0.0);

    if is_zero {
        return Err(EvalError::DivisionByZero { span });
    }

    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a
            .checked_div(b)
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a
            .checked_div(b)
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a
            .checked_div(b)
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a
            .checked_div(b)
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a
            .checked_div(b)
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a
            .checked_div(b)
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a
            .checked_div(b)
            .map(Value::U8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a
            .checked_div(b)
            .map(Value::U16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a
            .checked_div(b)
            .map(Value::U32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a
            .checked_div(b)
            .map(Value::U64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a
            .checked_div(b)
            .map(Value::U128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a
            .checked_div(b)
            .map(Value::Usize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a / b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a / b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "/".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_rem(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    // Check for division by zero (remainder by zero)
    let is_zero = matches!(
        &right,
        Value::I8(0)
            | Value::I16(0)
            | Value::I32(0)
            | Value::I64(0)
            | Value::I128(0)
            | Value::Isize(0)
            | Value::U8(0)
            | Value::U16(0)
            | Value::U32(0)
            | Value::U64(0)
            | Value::U128(0)
            | Value::Usize(0)
    );

    if is_zero {
        return Err(EvalError::DivisionByZero { span });
    }

    match (left, right) {
        (Value::I8(a), Value::I8(b)) => a
            .checked_rem(b)
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I16(a), Value::I16(b)) => a
            .checked_rem(b)
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I32(a), Value::I32(b)) => a
            .checked_rem(b)
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I64(a), Value::I64(b)) => a
            .checked_rem(b)
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::I128(a), Value::I128(b)) => a
            .checked_rem(b)
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Isize(a), Value::Isize(b)) => a
            .checked_rem(b)
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U8(a), Value::U8(b)) => a
            .checked_rem(b)
            .map(Value::U8)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U16(a), Value::U16(b)) => a
            .checked_rem(b)
            .map(Value::U16)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U32(a), Value::U32(b)) => a
            .checked_rem(b)
            .map(Value::U32)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U64(a), Value::U64(b)) => a
            .checked_rem(b)
            .map(Value::U64)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::U128(a), Value::U128(b)) => a
            .checked_rem(b)
            .map(Value::U128)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::Usize(a), Value::Usize(b)) => a
            .checked_rem(b)
            .map(Value::Usize)
            .ok_or(EvalError::IntegerOverflow { span }),
        (Value::F32(a), Value::F32(b)) => Ok(Value::F32(a % b)),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a % b)),
        (left, right) => Err(EvalError::InvalidBinaryOperands {
            op: "%".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Comparison Operations
// ═══════════════════════════════════════════════════════════════════════

macro_rules! impl_comparison {
    ($name:ident, $op:tt, $op_str:expr) => {
        fn $name(
            left: Value,
            right: Value,
            span: Option<proc_macro2::Span>,
        ) -> Result<Value, EvalError> {
            match (&left, &right) {
                // Integers
                (Value::I8(a), Value::I8(b)) => Ok(Value::Bool(a $op b)),
                (Value::I16(a), Value::I16(b)) => Ok(Value::Bool(a $op b)),
                (Value::I32(a), Value::I32(b)) => Ok(Value::Bool(a $op b)),
                (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(a $op b)),
                (Value::I128(a), Value::I128(b)) => Ok(Value::Bool(a $op b)),
                (Value::Isize(a), Value::Isize(b)) => Ok(Value::Bool(a $op b)),
                (Value::U8(a), Value::U8(b)) => Ok(Value::Bool(a $op b)),
                (Value::U16(a), Value::U16(b)) => Ok(Value::Bool(a $op b)),
                (Value::U32(a), Value::U32(b)) => Ok(Value::Bool(a $op b)),
                (Value::U64(a), Value::U64(b)) => Ok(Value::Bool(a $op b)),
                (Value::U128(a), Value::U128(b)) => Ok(Value::Bool(a $op b)),
                (Value::Usize(a), Value::Usize(b)) => Ok(Value::Bool(a $op b)),

                // Floats
                (Value::F32(a), Value::F32(b)) => Ok(Value::Bool(a $op b)),
                (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(a $op b)),

                // Chars
                (Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a $op b)),

                // Strings
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a $op b)),

                _ => Err(EvalError::InvalidBinaryOperands {
                    op: $op_str.to_string(),
                    left_type: type_name(&left).to_string(),
                    right_type: type_name(&right).to_string(),
                    span,
                }),
            }
        }
    };
}

impl_comparison!(eval_lt, <, "<");
impl_comparison!(eval_le, <=, "<=");
impl_comparison!(eval_gt, >, ">");
impl_comparison!(eval_ge, >=, ">=");

// ═══════════════════════════════════════════════════════════════════════
// Bitwise Operations
// ═══════════════════════════════════════════════════════════════════════

macro_rules! impl_bitwise {
    ($name:ident, $op:tt, $op_str:expr) => {
        fn $name(
            left: Value,
            right: Value,
            span: Option<proc_macro2::Span>,
        ) -> Result<Value, EvalError> {
            match (left, right) {
                // Booleans (logical operation)
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a $op b)),

                // Integers
                (Value::I8(a), Value::I8(b)) => Ok(Value::I8(a $op b)),
                (Value::I16(a), Value::I16(b)) => Ok(Value::I16(a $op b)),
                (Value::I32(a), Value::I32(b)) => Ok(Value::I32(a $op b)),
                (Value::I64(a), Value::I64(b)) => Ok(Value::I64(a $op b)),
                (Value::I128(a), Value::I128(b)) => Ok(Value::I128(a $op b)),
                (Value::Isize(a), Value::Isize(b)) => Ok(Value::Isize(a $op b)),
                (Value::U8(a), Value::U8(b)) => Ok(Value::U8(a $op b)),
                (Value::U16(a), Value::U16(b)) => Ok(Value::U16(a $op b)),
                (Value::U32(a), Value::U32(b)) => Ok(Value::U32(a $op b)),
                (Value::U64(a), Value::U64(b)) => Ok(Value::U64(a $op b)),
                (Value::U128(a), Value::U128(b)) => Ok(Value::U128(a $op b)),
                (Value::Usize(a), Value::Usize(b)) => Ok(Value::Usize(a $op b)),

                (left, right) => Err(EvalError::InvalidBinaryOperands {
                    op: $op_str.to_string(),
                    left_type: type_name(&left).to_string(),
                    right_type: type_name(&right).to_string(),
                    span,
                }),
            }
        }
    };
}

impl_bitwise!(eval_bitand, &, "&");
impl_bitwise!(eval_bitor, |, "|");
impl_bitwise!(eval_bitxor, ^, "^");

fn eval_shl(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    // Right side must be u32 for shift amount
    let shift = match &right {
        Value::I8(n) => *n as u32,
        Value::I16(n) => *n as u32,
        Value::I32(n) => *n as u32,
        Value::I64(n) => *n as u32,
        Value::U8(n) => *n as u32,
        Value::U16(n) => *n as u32,
        Value::U32(n) => *n,
        Value::U64(n) => *n as u32,
        Value::Usize(n) => *n as u32,
        _ => {
            return Err(EvalError::InvalidBinaryOperands {
                op: "<<".to_string(),
                left_type: type_name(&left).to_string(),
                right_type: type_name(&right).to_string(),
                span,
            });
        }
    };

    match left {
        Value::I8(a) => a
            .checked_shl(shift)
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I16(a) => a
            .checked_shl(shift)
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I32(a) => a
            .checked_shl(shift)
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I64(a) => a
            .checked_shl(shift)
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I128(a) => a
            .checked_shl(shift)
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::Isize(a) => a
            .checked_shl(shift)
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U8(a) => a
            .checked_shl(shift)
            .map(Value::U8)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U16(a) => a
            .checked_shl(shift)
            .map(Value::U16)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U32(a) => a
            .checked_shl(shift)
            .map(Value::U32)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U64(a) => a
            .checked_shl(shift)
            .map(Value::U64)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U128(a) => a
            .checked_shl(shift)
            .map(Value::U128)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::Usize(a) => a
            .checked_shl(shift)
            .map(Value::Usize)
            .ok_or(EvalError::IntegerOverflow { span }),
        _ => Err(EvalError::InvalidBinaryOperands {
            op: "<<".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}

fn eval_shr(
    left: Value,
    right: Value,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    let shift = match &right {
        Value::I8(n) => *n as u32,
        Value::I16(n) => *n as u32,
        Value::I32(n) => *n as u32,
        Value::I64(n) => *n as u32,
        Value::U8(n) => *n as u32,
        Value::U16(n) => *n as u32,
        Value::U32(n) => *n,
        Value::U64(n) => *n as u32,
        Value::Usize(n) => *n as u32,
        _ => {
            return Err(EvalError::InvalidBinaryOperands {
                op: ">>".to_string(),
                left_type: type_name(&left).to_string(),
                right_type: type_name(&right).to_string(),
                span,
            });
        }
    };

    match left {
        Value::I8(a) => a
            .checked_shr(shift)
            .map(Value::I8)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I16(a) => a
            .checked_shr(shift)
            .map(Value::I16)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I32(a) => a
            .checked_shr(shift)
            .map(Value::I32)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I64(a) => a
            .checked_shr(shift)
            .map(Value::I64)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::I128(a) => a
            .checked_shr(shift)
            .map(Value::I128)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::Isize(a) => a
            .checked_shr(shift)
            .map(Value::Isize)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U8(a) => a
            .checked_shr(shift)
            .map(Value::U8)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U16(a) => a
            .checked_shr(shift)
            .map(Value::U16)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U32(a) => a
            .checked_shr(shift)
            .map(Value::U32)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U64(a) => a
            .checked_shr(shift)
            .map(Value::U64)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::U128(a) => a
            .checked_shr(shift)
            .map(Value::U128)
            .ok_or(EvalError::IntegerOverflow { span }),
        Value::Usize(a) => a
            .checked_shr(shift)
            .map(Value::Usize)
            .ok_or(EvalError::IntegerOverflow { span }),
        _ => Err(EvalError::InvalidBinaryOperands {
            op: ">>".to_string(),
            left_type: type_name(&left).to_string(),
            right_type: type_name(&right).to_string(),
            span,
        }),
    }
}
