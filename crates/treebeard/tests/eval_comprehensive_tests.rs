// Additional comprehensive tests to reach 90%+ coverage
use treebeard::*;

fn eval(src: &str) -> std::result::Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    expr.eval(&mut env, &ctx)
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Operations - Complete Type Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_add_i64_i128_isize() {
    assert_eq!(eval("10i64 + 20i64").unwrap(), Value::I64(30));
    assert_eq!(eval("10i128 + 20i128").unwrap(), Value::I128(30));
    assert_eq!(eval("10isize + 20isize").unwrap(), Value::Isize(30));
}

#[test]
fn test_add_u64_u128_usize() {
    assert_eq!(eval("10u64 + 20u64").unwrap(), Value::U64(30));
    assert_eq!(eval("10u128 + 20u128").unwrap(), Value::U128(30));
    assert_eq!(eval("10usize + 20usize").unwrap(), Value::Usize(30));
}

#[test]
fn test_sub_i64_i128_isize() {
    assert_eq!(eval("30i64 - 10i64").unwrap(), Value::I64(20));
    assert_eq!(eval("30i128 - 10i128").unwrap(), Value::I128(20));
    assert_eq!(eval("30isize - 10isize").unwrap(), Value::Isize(20));
}

#[test]
fn test_sub_u64_u128_usize() {
    assert_eq!(eval("30u64 - 10u64").unwrap(), Value::U64(20));
    assert_eq!(eval("30u128 - 10u128").unwrap(), Value::U128(20));
    assert_eq!(eval("30usize - 10usize").unwrap(), Value::Usize(20));
}

#[test]
fn test_mul_i64_i128_isize() {
    assert_eq!(eval("3i64 * 4i64").unwrap(), Value::I64(12));
    assert_eq!(eval("3i128 * 4i128").unwrap(), Value::I128(12));
    assert_eq!(eval("3isize * 4isize").unwrap(), Value::Isize(12));
}

#[test]
fn test_mul_u64_u128_usize() {
    assert_eq!(eval("3u64 * 4u64").unwrap(), Value::U64(12));
    assert_eq!(eval("3u128 * 4u128").unwrap(), Value::U128(12));
    assert_eq!(eval("3usize * 4usize").unwrap(), Value::Usize(12));
}

#[test]
fn test_div_i64_i128_isize() {
    assert_eq!(eval("12i64 / 3i64").unwrap(), Value::I64(4));
    assert_eq!(eval("12i128 / 3i128").unwrap(), Value::I128(4));
    assert_eq!(eval("12isize / 3isize").unwrap(), Value::Isize(4));
}

#[test]
fn test_div_u64_u128_usize() {
    assert_eq!(eval("12u64 / 3u64").unwrap(), Value::U64(4));
    assert_eq!(eval("12u128 / 3u128").unwrap(), Value::U128(4));
    assert_eq!(eval("12usize / 3usize").unwrap(), Value::Usize(4));
}

#[test]
fn test_rem_i64_i128_isize() {
    assert_eq!(eval("10i64 % 3i64").unwrap(), Value::I64(1));
    assert_eq!(eval("10i128 % 3i128").unwrap(), Value::I128(1));
    assert_eq!(eval("10isize % 3isize").unwrap(), Value::Isize(1));
}

#[test]
fn test_rem_u64_u128_usize() {
    assert_eq!(eval("10u64 % 3u64").unwrap(), Value::U64(1));
    assert_eq!(eval("10u128 % 3u128").unwrap(), Value::U128(1));
    assert_eq!(eval("10usize % 3usize").unwrap(), Value::Usize(1));
}

#[test]
fn test_overflow_all_signed_sub() {
    assert!(matches!(
        eval("-128i8 - 1i8"),
        Err(EvalError::IntegerOverflow { .. })
    ));
    assert!(matches!(
        eval("-32768i16 - 1i16"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

#[test]
fn test_overflow_all_signed_mul() {
    assert!(matches!(
        eval("64i8 * 2i8"),
        Err(EvalError::IntegerOverflow { .. })
    ));
    assert!(matches!(
        eval("16384i16 * 2i16"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

#[test]
fn test_overflow_unsigned_mul() {
    assert!(matches!(
        eval("200u8 * 2u8"),
        Err(EvalError::IntegerOverflow { .. })
    ));
    assert!(matches!(
        eval("40000u16 * 2u16"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// Comparison - Complete Type Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_lt_i64_i128_isize() {
    assert_eq!(eval("1i64 < 2i64").unwrap(), Value::Bool(true));
    assert_eq!(eval("1i128 < 2i128").unwrap(), Value::Bool(true));
    assert_eq!(eval("1isize < 2isize").unwrap(), Value::Bool(true));
}

#[test]
fn test_lt_u64_u128_usize() {
    assert_eq!(eval("1u64 < 2u64").unwrap(), Value::Bool(true));
    assert_eq!(eval("1u128 < 2u128").unwrap(), Value::Bool(true));
    assert_eq!(eval("1usize < 2usize").unwrap(), Value::Bool(true));
}

#[test]
fn test_le_i64_i128_isize() {
    assert_eq!(eval("2i64 <= 2i64").unwrap(), Value::Bool(true));
    assert_eq!(eval("2i128 <= 2i128").unwrap(), Value::Bool(true));
    assert_eq!(eval("2isize <= 2isize").unwrap(), Value::Bool(true));
}

#[test]
fn test_le_u64_u128_usize() {
    assert_eq!(eval("2u64 <= 2u64").unwrap(), Value::Bool(true));
    assert_eq!(eval("2u128 <= 2u128").unwrap(), Value::Bool(true));
    assert_eq!(eval("2usize <= 2usize").unwrap(), Value::Bool(true));
}

#[test]
fn test_gt_i64_i128_isize() {
    assert_eq!(eval("2i64 > 1i64").unwrap(), Value::Bool(true));
    assert_eq!(eval("2i128 > 1i128").unwrap(), Value::Bool(true));
    assert_eq!(eval("2isize > 1isize").unwrap(), Value::Bool(true));
}

#[test]
fn test_gt_u64_u128_usize() {
    assert_eq!(eval("2u64 > 1u64").unwrap(), Value::Bool(true));
    assert_eq!(eval("2u128 > 1u128").unwrap(), Value::Bool(true));
    assert_eq!(eval("2usize > 1usize").unwrap(), Value::Bool(true));
}

#[test]
fn test_ge_i64_i128_isize() {
    assert_eq!(eval("2i64 >= 2i64").unwrap(), Value::Bool(true));
    assert_eq!(eval("2i128 >= 2i128").unwrap(), Value::Bool(true));
    assert_eq!(eval("2isize >= 2isize").unwrap(), Value::Bool(true));
}

#[test]
fn test_ge_u64_u128_usize() {
    assert_eq!(eval("2u64 >= 2u64").unwrap(), Value::Bool(true));
    assert_eq!(eval("2u128 >= 2u128").unwrap(), Value::Bool(true));
    assert_eq!(eval("2usize >= 2usize").unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Bitwise - Complete Type Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_bitwise_i64_i128_isize() {
    assert_eq!(eval("12i64 & 10i64").unwrap(), Value::I64(8));
    assert_eq!(eval("12i128 | 10i128").unwrap(), Value::I128(14));
    assert_eq!(eval("12isize ^ 10isize").unwrap(), Value::Isize(6));
}

#[test]
fn test_bitwise_u64_u128_usize() {
    assert_eq!(eval("12u64 & 10u64").unwrap(), Value::U64(8));
    assert_eq!(eval("12u128 | 10u128").unwrap(), Value::U128(14));
    assert_eq!(eval("12usize ^ 10usize").unwrap(), Value::Usize(6));
}

#[test]
fn test_shift_i64_i128_isize() {
    assert_eq!(eval("1i64 << 4u32").unwrap(), Value::I64(16));
    assert_eq!(eval("1i128 << 4u32").unwrap(), Value::I128(16));
    assert_eq!(eval("1isize << 4u32").unwrap(), Value::Isize(16));
    assert_eq!(eval("16i64 >> 2u32").unwrap(), Value::I64(4));
    assert_eq!(eval("16i128 >> 2u32").unwrap(), Value::I128(4));
    assert_eq!(eval("16isize >> 2u32").unwrap(), Value::Isize(4));
}

#[test]
fn test_shift_u64_u128_usize() {
    assert_eq!(eval("1u64 << 4u32").unwrap(), Value::U64(16));
    assert_eq!(eval("1u128 << 4u32").unwrap(), Value::U128(16));
    assert_eq!(eval("1usize << 4u32").unwrap(), Value::Usize(16));
    assert_eq!(eval("16u64 >> 2u32").unwrap(), Value::U64(4));
    assert_eq!(eval("16u128 >> 2u32").unwrap(), Value::U128(4));
    assert_eq!(eval("16usize >> 2u32").unwrap(), Value::Usize(4));
}

#[test]
fn test_shift_right_with_all_shift_types() {
    assert_eq!(eval("16 >> 1i8").unwrap(), Value::I64(8));
    assert_eq!(eval("16 >> 1i16").unwrap(), Value::I64(8));
    assert_eq!(eval("16 >> 1i32").unwrap(), Value::I64(8));
    assert_eq!(eval("16 >> 1i64").unwrap(), Value::I64(8));
    assert_eq!(eval("16 >> 1u8").unwrap(), Value::I64(8));
    assert_eq!(eval("16 >> 1u16").unwrap(), Value::I64(8));
    assert_eq!(eval("16 >> 1u64").unwrap(), Value::I64(8));
    assert_eq!(eval("16 >> 1usize").unwrap(), Value::I64(8));
}

// ═══════════════════════════════════════════════════════════════════════
// Unary - Complete Type Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_neg_i64_i128_isize() {
    assert_eq!(eval("-10i64").unwrap(), Value::I64(-10));
    assert_eq!(eval("-10i128").unwrap(), Value::I128(-10));
    assert_eq!(eval("-10isize").unwrap(), Value::Isize(-10));
}

#[test]
fn test_not_i64_i128_isize() {
    assert_eq!(eval("!0i64").unwrap(), Value::I64(-1));
    assert_eq!(eval("!0i128").unwrap(), Value::I128(-1));
    assert_eq!(eval("!0isize").unwrap(), Value::Isize(-1));
}

#[test]
fn test_not_u64_u128_usize() {
    assert_eq!(eval("!0u64").unwrap(), Value::U64(u64::MAX));
    assert_eq!(eval("!0u128").unwrap(), Value::U128(u128::MAX));
    // usize depends on platform, just check it compiles
    let result = eval("!0usize");
    assert!(result.is_ok());
}

#[test]
fn test_neg_overflow_i16() {
    assert!(matches!(
        eval("-(-32768i16)"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

#[test]
fn test_neg_overflow_i32() {
    assert!(matches!(
        eval("-(-2147483648i32)"),
        Err(EvalError::IntegerOverflow { .. })
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// Literal - Complete Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_literal_u64_u128() {
    assert_eq!(eval("42u64").unwrap(), Value::U64(42));
    assert_eq!(eval("42u128").unwrap(), Value::U128(42));
}

#[test]
fn test_literal_i128() {
    assert_eq!(eval("42i128").unwrap(), Value::I128(42));
}

#[test]
fn test_literal_float_edge_cases() {
    assert_eq!(eval("0.0f32").unwrap(), Value::F32(0.0));
    assert_eq!(eval("0.0f64").unwrap(), Value::F64(0.0));
    assert_eq!(eval("1.0").unwrap(), Value::F64(1.0));
}

// ═══════════════════════════════════════════════════════════════════════
// All "Not Yet Implemented" Expression Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_unsupported_array() {
    let result = eval("[1, 2, 3]");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_assign() {
    let result = eval("x = 5");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_async() {
    let result = eval("async { 42 }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_await() {
    let result = eval("foo.await");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_block() {
    let result = eval("{ 42 }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_break() {
    let result = eval("break");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_cast() {
    let result = eval("1 as u32");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_closure() {
    let result = eval("|x| x + 1");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_const_block() {
    let result = eval("const { 42 }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_continue() {
    let result = eval("continue");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_field_access() {
    let result = eval("x.field");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_for_loop() {
    let result = eval("for i in 0..10 { }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_index() {
    let result = eval("arr[0]");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_let_guard() {
    let result = eval("let x = 5");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_macro() {
    let result = eval("println!(\"hello\")");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_range() {
    let result = eval("0..10");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_reference() {
    let result = eval("&x");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_repeat() {
    let result = eval("[0; 10]");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_return() {
    let result = eval("return 42");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_struct_literal() {
    let result = eval("Point { x: 1, y: 2 }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_try() {
    let result = eval("foo?");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_try_block() {
    let result = eval("try { 42 }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_tuple() {
    let result = eval("(1, 2, 3)");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_unsafe() {
    let result = eval("unsafe { 42 }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_while_loop() {
    let result = eval("while true { }");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

#[test]
fn test_unsupported_yield() {
    let result = eval("yield 42");
    assert!(matches!(result, Err(EvalError::UnsupportedExpr { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// NOTE: Assignment operators (+=, -=, etc.) are tested in binary.rs directly
// They can't be tested via eval() since they parse as statements, not expressions
// ═══════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════
// Division by Zero - All Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_div_by_zero_i64_i128_isize() {
    assert!(matches!(
        eval("1i64 / 0i64"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1i128 / 0i128"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1isize / 0isize"),
        Err(EvalError::DivisionByZero { .. })
    ));
}

#[test]
fn test_div_by_zero_u64_u128_usize() {
    assert!(matches!(
        eval("1u64 / 0u64"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1u128 / 0u128"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1usize / 0usize"),
        Err(EvalError::DivisionByZero { .. })
    ));
}

#[test]
fn test_rem_by_zero_i64_i128_isize() {
    assert!(matches!(
        eval("1i64 % 0i64"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1i128 % 0i128"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1isize % 0isize"),
        Err(EvalError::DivisionByZero { .. })
    ));
}

#[test]
fn test_rem_by_zero_u64_u128_usize() {
    assert!(matches!(
        eval("1u64 % 0u64"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1u128 % 0u128"),
        Err(EvalError::DivisionByZero { .. })
    ));
    assert!(matches!(
        eval("1usize % 0usize"),
        Err(EvalError::DivisionByZero { .. })
    ));
}
