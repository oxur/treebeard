// Comprehensive coverage tests for eval modules
use treebeard::*;

fn eval(src: &str) -> std::result::Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx);

    // Convert stray ControlFlow errors to appropriate errors
    match result {
        Err(EvalError::ControlFlow(cf)) => match cf {
            ControlFlow::Break { .. } => Err(EvalError::BreakOutsideLoop { span: None }),
            ControlFlow::Continue { .. } => Err(EvalError::ContinueOutsideLoop { span: None }),
            ControlFlow::Return { .. } => Err(EvalError::ReturnOutsideFunction { span: None }),
        },
        other => other,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Context Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_context_new() {
    let ctx = EvalContext::new();
    assert_eq!(ctx.max_call_depth, 1000);
    assert!(!ctx.is_interrupted());
    assert!(!ctx.trace);
}

#[test]
fn test_eval_context_with_max_call_depth() {
    let ctx = EvalContext::with_max_call_depth(500);
    assert_eq!(ctx.max_call_depth, 500);
}

#[test]
fn test_eval_context_interrupt_and_reset() {
    let ctx = EvalContext::default();
    assert!(!ctx.is_interrupted());
    ctx.interrupt();
    assert!(ctx.is_interrupted());
    ctx.reset_interrupt();
    assert!(!ctx.is_interrupted());
}

// ═══════════════════════════════════════════════════════════════════════
// Literal Coverage - All Integer Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_literal_all_signed_integer_types() {
    assert_eq!(eval("10i8").unwrap(), Value::I8(10));
    assert_eq!(eval("10i16").unwrap(), Value::I16(10));
    assert_eq!(eval("10i32").unwrap(), Value::I32(10));
    assert_eq!(eval("10i64").unwrap(), Value::I64(10));
    assert_eq!(eval("10i128").unwrap(), Value::I128(10));
    assert_eq!(eval("10isize").unwrap(), Value::Isize(10));
}

#[test]
fn test_literal_all_unsigned_integer_types() {
    assert_eq!(eval("10u8").unwrap(), Value::U8(10));
    assert_eq!(eval("10u16").unwrap(), Value::U16(10));
    assert_eq!(eval("10u32").unwrap(), Value::U32(10));
    assert_eq!(eval("10u64").unwrap(), Value::U64(10));
    assert_eq!(eval("10u128").unwrap(), Value::U128(10));
    assert_eq!(eval("10usize").unwrap(), Value::Usize(10));
}

#[test]
fn test_literal_integer_overflow() {
    let result = eval("128i8");
    assert!(matches!(result, Err(EvalError::IntegerOverflow { .. })));
}

#[test]
fn test_literal_bytestring() {
    let result = eval(r#"b"hello""#);
    assert_eq!(result.unwrap(), Value::bytes(b"hello".to_vec()));
}

// ═══════════════════════════════════════════════════════════════════════
// Path Coverage - Error Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_path_qualified_path_unsupported() {
    let result = eval("std::vec::Vec");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_path_type_arguments_unsupported() {
    let result = eval("foo::<i32>");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Unary Coverage - All Integer Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_unary_neg_all_signed_types() {
    assert_eq!(eval("-10i8").unwrap(), Value::I8(-10));
    assert_eq!(eval("-10i16").unwrap(), Value::I16(-10));
    assert_eq!(eval("-10i32").unwrap(), Value::I32(-10));
    assert_eq!(eval("-10i64").unwrap(), Value::I64(-10));
    assert_eq!(eval("-10i128").unwrap(), Value::I128(-10));
    assert_eq!(eval("-10isize").unwrap(), Value::Isize(-10));
}

#[test]
fn test_unary_neg_floats() {
    assert_eq!(eval("-3.14f32").unwrap(), Value::F32(-3.14));
    assert_eq!(eval("-3.14f64").unwrap(), Value::F64(-3.14));
}

#[test]
fn test_unary_neg_unsigned_invalid() {
    assert!(matches!(
        eval("-10u8"),
        Err(EvalError::InvalidUnaryOperand { .. })
    ));
    assert!(matches!(
        eval("-10u32"),
        Err(EvalError::InvalidUnaryOperand { .. })
    ));
}

#[test]
fn test_unary_not_all_integer_types() {
    assert_eq!(eval("!0i8").unwrap(), Value::I8(-1));
    assert_eq!(eval("!0i16").unwrap(), Value::I16(-1));
    assert_eq!(eval("!0i32").unwrap(), Value::I32(-1));
    assert_eq!(eval("!0i64").unwrap(), Value::I64(-1));
    assert_eq!(eval("!0i128").unwrap(), Value::I128(-1));
    assert_eq!(eval("!0isize").unwrap(), Value::Isize(-1));
    assert_eq!(eval("!0u8").unwrap(), Value::U8(255));
    assert_eq!(eval("!0u16").unwrap(), Value::U16(65535));
    assert_eq!(eval("!0u32").unwrap(), Value::U32(4294967295));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Arithmetic - All Integer Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_binary_add_all_signed_types() {
    assert_eq!(eval("2i8 + 3i8").unwrap(), Value::I8(5));
    assert_eq!(eval("2i16 + 3i16").unwrap(), Value::I16(5));
    assert_eq!(eval("2i32 + 3i32").unwrap(), Value::I32(5));
    assert_eq!(eval("2i64 + 3i64").unwrap(), Value::I64(5));
    assert_eq!(eval("2i128 + 3i128").unwrap(), Value::I128(5));
    assert_eq!(eval("2isize + 3isize").unwrap(), Value::Isize(5));
}

#[test]
fn test_binary_add_all_unsigned_types() {
    assert_eq!(eval("2u8 + 3u8").unwrap(), Value::U8(5));
    assert_eq!(eval("2u16 + 3u16").unwrap(), Value::U16(5));
    assert_eq!(eval("2u32 + 3u32").unwrap(), Value::U32(5));
    assert_eq!(eval("2u64 + 3u64").unwrap(), Value::U64(5));
    assert_eq!(eval("2u128 + 3u128").unwrap(), Value::U128(5));
    assert_eq!(eval("2usize + 3usize").unwrap(), Value::Usize(5));
}

#[test]
fn test_binary_add_floats() {
    assert_eq!(eval("2.0f32 + 3.0f32").unwrap(), Value::F32(5.0));
    assert_eq!(eval("2.0f64 + 3.0f64").unwrap(), Value::F64(5.0));
}

#[test]
fn test_binary_sub_all_types() {
    assert_eq!(eval("5i8 - 2i8").unwrap(), Value::I8(3));
    assert_eq!(eval("5i16 - 2i16").unwrap(), Value::I16(3));
    assert_eq!(eval("5i32 - 2i32").unwrap(), Value::I32(3));
    assert_eq!(eval("5u8 - 2u8").unwrap(), Value::U8(3));
    assert_eq!(eval("5u16 - 2u16").unwrap(), Value::U16(3));
    assert_eq!(eval("5.0f32 - 2.0f32").unwrap(), Value::F32(3.0));
}

#[test]
fn test_binary_mul_all_types() {
    assert_eq!(eval("2i8 * 3i8").unwrap(), Value::I8(6));
    assert_eq!(eval("2i16 * 3i16").unwrap(), Value::I16(6));
    assert_eq!(eval("2i32 * 3i32").unwrap(), Value::I32(6));
    assert_eq!(eval("2u8 * 3u8").unwrap(), Value::U8(6));
    assert_eq!(eval("2u16 * 3u16").unwrap(), Value::U16(6));
    assert_eq!(eval("2.0f32 * 3.0f32").unwrap(), Value::F32(6.0));
}

#[test]
fn test_binary_div_all_types() {
    assert_eq!(eval("6i8 / 2i8").unwrap(), Value::I8(3));
    assert_eq!(eval("6i16 / 2i16").unwrap(), Value::I16(3));
    assert_eq!(eval("6i32 / 2i32").unwrap(), Value::I32(3));
    assert_eq!(eval("6u8 / 2u8").unwrap(), Value::U8(3));
    assert_eq!(eval("6u16 / 2u16").unwrap(), Value::U16(3));
    assert_eq!(eval("6.0f32 / 2.0f32").unwrap(), Value::F32(3.0));
}

#[test]
fn test_binary_rem_all_types() {
    assert_eq!(eval("7i8 % 3i8").unwrap(), Value::I8(1));
    assert_eq!(eval("7i16 % 3i16").unwrap(), Value::I16(1));
    assert_eq!(eval("7i32 % 3i32").unwrap(), Value::I32(1));
    assert_eq!(eval("7u8 % 3u8").unwrap(), Value::U8(1));
    assert_eq!(eval("7u16 % 3u16").unwrap(), Value::U16(1));
    assert_eq!(eval("7.0f32 % 3.0f32").unwrap(), Value::F32(1.0));
}

#[test]
fn test_binary_overflow_all_signed_types() {
    assert!(matches!(
        eval("127i8 + 1i8"),
        Err(EvalError::IntegerOverflow { .. })
    ));
    assert!(matches!(
        eval("32767i16 + 1i16"),
        Err(EvalError::IntegerOverflow { .. })
    ));
    assert!(matches!(
        eval("-128i8 - 1i8"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

#[test]
fn test_binary_overflow_unsigned_types() {
    assert!(matches!(
        eval("255u8 + 1u8"),
        Err(EvalError::IntegerOverflow { .. })
    ));
    assert!(matches!(
        eval("0u8 - 1u8"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

#[test]
fn test_binary_div_by_zero_all_types() {
    assert!(matches!(
        eval("1i8 / 0i8"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1i16 / 0i16"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1u8 / 0u8"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1.0f32 / 0.0f32"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1.0f64 / 0.0f64"),
        Err(EvalError::DivisionByZero { .. })
    ));
}

#[test]
fn test_binary_rem_by_zero() {
    assert!(matches!(
        eval("7i8 % 0i8"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("7u8 % 0u8"),
        Err(EvalError::DivisionByZero { .. })
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Comparison - All Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_binary_lt_all_integer_types() {
    assert_eq!(eval("1i8 < 2i8").unwrap(), Value::Bool(true));
    assert_eq!(eval("1i16 < 2i16").unwrap(), Value::Bool(true));
    assert_eq!(eval("1i32 < 2i32").unwrap(), Value::Bool(true));
    assert_eq!(eval("1u8 < 2u8").unwrap(), Value::Bool(true));
    assert_eq!(eval("1u16 < 2u16").unwrap(), Value::Bool(true));
}

#[test]
fn test_binary_lt_floats() {
    assert_eq!(eval("1.0f32 < 2.0f32").unwrap(), Value::Bool(true));
    assert_eq!(eval("1.0f64 < 2.0f64").unwrap(), Value::Bool(true));
}

#[test]
fn test_binary_lt_chars() {
    assert_eq!(eval("'a' < 'b'").unwrap(), Value::Bool(true));
}

#[test]
fn test_binary_le_all_types() {
    assert_eq!(eval("1i8 <= 1i8").unwrap(), Value::Bool(true));
    assert_eq!(eval("1i16 <= 2i16").unwrap(), Value::Bool(true));
    assert_eq!(eval("1u8 <= 1u8").unwrap(), Value::Bool(true));
    assert_eq!(eval("1.0f32 <= 2.0f32").unwrap(), Value::Bool(true));
}

#[test]
fn test_binary_gt_all_types() {
    assert_eq!(eval("2i8 > 1i8").unwrap(), Value::Bool(true));
    assert_eq!(eval("2i16 > 1i16").unwrap(), Value::Bool(true));
    assert_eq!(eval("2u8 > 1u8").unwrap(), Value::Bool(true));
    assert_eq!(eval("2.0f32 > 1.0f32").unwrap(), Value::Bool(true));
}

#[test]
fn test_binary_ge_all_types() {
    assert_eq!(eval("2i8 >= 1i8").unwrap(), Value::Bool(true));
    assert_eq!(eval("2i8 >= 2i8").unwrap(), Value::Bool(true));
    assert_eq!(eval("2u8 >= 1u8").unwrap(), Value::Bool(true));
    assert_eq!(eval("2.0f32 >= 2.0f32").unwrap(), Value::Bool(true));
}

#[test]
fn test_comparison_type_mismatch() {
    assert!(matches!(
        eval("1i8 < 2i16"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("1u8 < 2u16"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Bitwise - All Integer Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_bitwise_and_all_types() {
    assert_eq!(eval("0b1100i8 & 0b1010i8").unwrap(), Value::I8(0b1000));
    assert_eq!(eval("0b1100i16 & 0b1010i16").unwrap(), Value::I16(0b1000));
    assert_eq!(eval("0b1100i32 & 0b1010i32").unwrap(), Value::I32(0b1000));
    assert_eq!(eval("0b1100u8 & 0b1010u8").unwrap(), Value::U8(0b1000));
    assert_eq!(eval("0b1100u16 & 0b1010u16").unwrap(), Value::U16(0b1000));
}

#[test]
fn test_bitwise_or_all_types() {
    assert_eq!(eval("0b1100i8 | 0b1010i8").unwrap(), Value::I8(0b1110));
    assert_eq!(eval("0b1100u8 | 0b1010u8").unwrap(), Value::U8(0b1110));
}

#[test]
fn test_bitwise_xor_all_types() {
    assert_eq!(eval("0b1100i8 ^ 0b1010i8").unwrap(), Value::I8(0b0110));
    assert_eq!(eval("0b1100u8 ^ 0b1010u8").unwrap(), Value::U8(0b0110));
}

#[test]
fn test_bitwise_and_booleans() {
    assert_eq!(eval("true & false").unwrap(), Value::Bool(false));
    assert_eq!(eval("true & true").unwrap(), Value::Bool(true));
}

#[test]
fn test_shift_all_integer_types() {
    assert_eq!(eval("1i8 << 2u32").unwrap(), Value::I8(4));
    assert_eq!(eval("1i16 << 2u32").unwrap(), Value::I16(4));
    assert_eq!(eval("1i32 << 2u32").unwrap(), Value::I32(4));
    assert_eq!(eval("1u8 << 2u32").unwrap(), Value::U8(4));
    assert_eq!(eval("1u16 << 2u32").unwrap(), Value::U16(4));
}

#[test]
fn test_shift_right_all_types() {
    assert_eq!(eval("8i8 >> 2u32").unwrap(), Value::I8(2));
    assert_eq!(eval("8i16 >> 2u32").unwrap(), Value::I16(2));
    assert_eq!(eval("8u8 >> 2u32").unwrap(), Value::U8(2));
}

#[test]
fn test_shift_overflow() {
    assert!(matches!(
        eval("1i8 << 8u32"),
        Err(EvalError::IntegerOverflow { .. })
    ));
    assert!(matches!(
        eval("1u8 << 8u32"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

#[test]
fn test_shift_with_various_shift_types() {
    assert_eq!(eval("4 << 1i8").unwrap(), Value::I64(8));
    assert_eq!(eval("4 << 1i16").unwrap(), Value::I64(8));
    assert_eq!(eval("4 << 1i32").unwrap(), Value::I64(8));
    assert_eq!(eval("4 << 1i64").unwrap(), Value::I64(8));
    assert_eq!(eval("4 << 1u8").unwrap(), Value::I64(8));
    assert_eq!(eval("4 << 1u16").unwrap(), Value::I64(8));
    assert_eq!(eval("4 << 1u64").unwrap(), Value::I64(8));
    assert_eq!(eval("4 << 1usize").unwrap(), Value::I64(8));
}

// ═══════════════════════════════════════════════════════════════════════
// Unsupported Expressions
// ═══════════════════════════════════════════════════════════════════════

// if, match, and loop are now supported in Stage 1.4
#[test]
fn test_if_expr_basic() {
    let result = eval("if true { 1 } else { 2 }");
    assert!(matches!(result, Ok(Value::I64(1))));
}

#[test]
fn test_match_expr_basic() {
    let result = eval("match 1 { 1 => 2, _ => 3 }");
    assert!(matches!(result, Ok(Value::I64(2))));
}

#[test]
fn test_loop_expr_with_break() {
    let result = eval("loop { break; }");
    assert!(matches!(result, Ok(Value::Unit)));
}

#[test]
fn test_unsupported_function_call() {
    // Function calls are now supported (Stage 1.5), but undefined function is an error
    let result = eval("foo()");
    assert!(matches!(result, Err(EvalError::UndefinedVariable { .. })));
}

#[test]
fn test_unsupported_method_call() {
    // Method calls are now supported (Stage 1.5), but undefined variable is an error
    let result = eval("x.foo()");
    assert!(matches!(result, Err(EvalError::UndefinedVariable { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Error Message Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_error_has_span() {
    let result = eval("undefined");
    if let Err(err) = result {
        assert!(err.span().is_some());
    }
}

#[test]
fn test_logical_operator_type_error() {
    let result = eval("1 && 2");
    assert!(matches!(
        result,
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
}

#[test]
fn test_arithmetic_invalid_operands() {
    assert!(matches!(
        eval("true + false"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("true - false"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("true * false"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("true / false"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("true % false"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
}

#[test]
fn test_bitwise_invalid_operands() {
    assert!(matches!(
        eval("1.0 & 2.0"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("1.0 | 2.0"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("1.0 ^ 2.0"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
}

#[test]
fn test_shift_invalid_operands() {
    assert!(matches!(
        eval("1.0 << 2"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
    assert!(matches!(
        eval("1 << true"),
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// Additional Integer Type Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_i64_operations() {
    assert_eq!(eval("100i64 + 200i64").unwrap(), Value::I64(300));
    assert_eq!(eval("100i64 - 50i64").unwrap(), Value::I64(50));
    assert_eq!(eval("10i64 * 5i64").unwrap(), Value::I64(50));
    assert_eq!(eval("100i64 / 4i64").unwrap(), Value::I64(25));
    assert_eq!(eval("100i64 % 7i64").unwrap(), Value::I64(2));
}

#[test]
fn test_i128_operations() {
    assert_eq!(eval("100i128 + 200i128").unwrap(), Value::I128(300));
    assert_eq!(eval("100i128 - 50i128").unwrap(), Value::I128(50));
}

#[test]
fn test_u64_operations() {
    assert_eq!(eval("100u64 + 200u64").unwrap(), Value::U64(300));
    assert_eq!(eval("200u64 - 50u64").unwrap(), Value::U64(150));
}

#[test]
fn test_u128_operations() {
    assert_eq!(eval("100u128 + 200u128").unwrap(), Value::U128(300));
    assert_eq!(eval("200u128 - 50u128").unwrap(), Value::U128(150));
}

#[test]
fn test_usize_operations() {
    assert_eq!(eval("10usize + 20usize").unwrap(), Value::Usize(30));
    assert_eq!(eval("20usize - 10usize").unwrap(), Value::Usize(10));
}
