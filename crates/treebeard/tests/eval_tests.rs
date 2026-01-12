use treebeard::*;

// Helper to parse and evaluate an expression
fn eval(src: &str) -> std::result::Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    expr.eval(&mut env, &ctx)
}

// Helper with pre-defined environment
fn eval_with_env(src: &str, env: &mut Environment) -> std::result::Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let ctx = EvalContext::default();
    expr.eval(env, &ctx)
}

// ═══════════════════════════════════════════════════════════════════════
// Literal Evaluation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_lit_integer() {
    assert_eq!(eval("42").unwrap(), Value::I64(42));
    assert_eq!(eval("0").unwrap(), Value::I64(0));
    assert_eq!(eval("-1").unwrap(), Value::I64(-1));
}

#[test]
fn test_eval_lit_integer_suffixes() {
    assert_eq!(eval("42i8").unwrap(), Value::I8(42));
    assert_eq!(eval("42i16").unwrap(), Value::I16(42));
    assert_eq!(eval("42i32").unwrap(), Value::I32(42));
    assert_eq!(eval("42i64").unwrap(), Value::I64(42));
    assert_eq!(eval("42u8").unwrap(), Value::U8(42));
    assert_eq!(eval("42u32").unwrap(), Value::U32(42));
    assert_eq!(eval("42usize").unwrap(), Value::Usize(42));
}

#[test]
fn test_eval_lit_float() {
    assert_eq!(eval("3.14").unwrap(), Value::F64(3.14));
    assert_eq!(eval("3.14f32").unwrap(), Value::F32(3.14));
    assert_eq!(eval("3.14f64").unwrap(), Value::F64(3.14));
}

#[test]
fn test_eval_lit_bool() {
    assert_eq!(eval("true").unwrap(), Value::Bool(true));
    assert_eq!(eval("false").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_lit_char() {
    assert_eq!(eval("'a'").unwrap(), Value::Char('a'));
    assert_eq!(eval("'\\n'").unwrap(), Value::Char('\n'));
}

#[test]
fn test_eval_lit_string() {
    assert_eq!(eval(r#""hello""#).unwrap(), Value::string("hello"));
    assert_eq!(
        eval(r#""hello\nworld""#).unwrap(),
        Value::string("hello\nworld")
    );
}

#[test]
fn test_eval_lit_byte() {
    assert_eq!(eval("b'a'").unwrap(), Value::U8(b'a'));
}

// ═══════════════════════════════════════════════════════════════════════
// Path Evaluation (Variable Lookup)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_path_defined() {
    let mut env = Environment::new();
    env.define("x", Value::I64(42));

    assert_eq!(eval_with_env("x", &mut env).unwrap(), Value::I64(42));
}

#[test]
fn test_eval_path_undefined() {
    let result = eval("undefined_var");
    assert!(matches!(result, Err(EvalError::UndefinedVariable { .. })));
}

#[test]
fn test_eval_path_shadowing() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));
    env.push_frame();
    env.define("x", Value::I64(2));

    assert_eq!(eval_with_env("x", &mut env).unwrap(), Value::I64(2));
}

// ═══════════════════════════════════════════════════════════════════════
// Unary Operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_unary_neg() {
    assert_eq!(eval("-42").unwrap(), Value::I64(-42));
    assert_eq!(eval("-3.14").unwrap(), Value::F64(-3.14));
}

#[test]
fn test_eval_unary_neg_overflow() {
    // i8::MIN cannot be negated
    let result = eval("-(-128i8)");
    assert!(matches!(result, Err(EvalError::IntegerOverflow { .. })));
}

#[test]
fn test_eval_unary_not_bool() {
    assert_eq!(eval("!true").unwrap(), Value::Bool(false));
    assert_eq!(eval("!false").unwrap(), Value::Bool(true));
}

#[test]
fn test_eval_unary_not_bitwise() {
    assert_eq!(eval("!0u8").unwrap(), Value::U8(255));
    assert_eq!(eval("!0i32").unwrap(), Value::I32(-1));
}

#[test]
fn test_eval_unary_invalid() {
    let result = eval("-true");
    assert!(matches!(result, Err(EvalError::InvalidUnaryOperand { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Arithmetic
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_add() {
    assert_eq!(eval("1 + 2").unwrap(), Value::I64(3));
    assert_eq!(eval("1.5 + 2.5").unwrap(), Value::F64(4.0));
}

#[test]
fn test_eval_binary_add_string() {
    assert_eq!(
        eval(r#""hello" + " world""#).unwrap(),
        Value::string("hello world")
    );
}

#[test]
fn test_eval_binary_sub() {
    assert_eq!(eval("5 - 3").unwrap(), Value::I64(2));
    assert_eq!(eval("3.5 - 1.5").unwrap(), Value::F64(2.0));
}

#[test]
fn test_eval_binary_mul() {
    assert_eq!(eval("3 * 4").unwrap(), Value::I64(12));
    assert_eq!(eval("2.0 * 3.0").unwrap(), Value::F64(6.0));
}

#[test]
fn test_eval_binary_div() {
    assert_eq!(eval("10 / 3").unwrap(), Value::I64(3)); // Integer division
    assert_eq!(eval("10.0 / 4.0").unwrap(), Value::F64(2.5));
}

#[test]
fn test_eval_binary_div_by_zero() {
    let result = eval("1 / 0");
    assert!(matches!(result, Err(EvalError::DivisionByZero { .. })));
}

#[test]
fn test_eval_binary_rem() {
    assert_eq!(eval("10 % 3").unwrap(), Value::I64(1));
    assert_eq!(eval("10 % 5").unwrap(), Value::I64(0));
}

#[test]
fn test_eval_binary_overflow() {
    let result = eval("127i8 + 1i8");
    assert!(matches!(result, Err(EvalError::IntegerOverflow { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Comparison
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_eq() {
    assert_eq!(eval("1 == 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 == 2").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_ne() {
    assert_eq!(eval("1 != 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 != 1").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_lt() {
    assert_eq!(eval("1 < 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("2 < 1").unwrap(), Value::Bool(false));
    assert_eq!(eval("1 < 1").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_le() {
    assert_eq!(eval("1 <= 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 <= 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("2 <= 1").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_gt() {
    assert_eq!(eval("2 > 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 > 2").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_ge() {
    assert_eq!(eval("2 >= 1").unwrap(), Value::Bool(true));
    assert_eq!(eval("2 >= 2").unwrap(), Value::Bool(true));
    assert_eq!(eval("1 >= 2").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_string_comparison() {
    assert_eq!(eval(r#""a" < "b""#).unwrap(), Value::Bool(true));
    assert_eq!(eval(r#""abc" == "abc""#).unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Logical (Short-Circuit)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_and() {
    assert_eq!(eval("true && true").unwrap(), Value::Bool(true));
    assert_eq!(eval("true && false").unwrap(), Value::Bool(false));
    assert_eq!(eval("false && true").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_or() {
    assert_eq!(eval("true || false").unwrap(), Value::Bool(true));
    assert_eq!(eval("false || true").unwrap(), Value::Bool(true));
    assert_eq!(eval("false || false").unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_and_short_circuit() {
    // This would fail if not short-circuiting (undefined_var doesn't exist)
    let mut env = Environment::new();
    let result = eval_with_env("false && undefined_var", &mut env);
    assert_eq!(result.unwrap(), Value::Bool(false));
}

#[test]
fn test_eval_binary_or_short_circuit() {
    let mut env = Environment::new();
    let result = eval_with_env("true || undefined_var", &mut env);
    assert_eq!(result.unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Binary Bitwise
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_bitand() {
    assert_eq!(eval("0b1100u8 & 0b1010u8").unwrap(), Value::U8(0b1000));
}

#[test]
fn test_eval_binary_bitor() {
    assert_eq!(eval("0b1100u8 | 0b1010u8").unwrap(), Value::U8(0b1110));
}

#[test]
fn test_eval_binary_bitxor() {
    assert_eq!(eval("0b1100u8 ^ 0b1010u8").unwrap(), Value::U8(0b0110));
}

#[test]
fn test_eval_binary_shl() {
    assert_eq!(eval("1 << 4").unwrap(), Value::I64(16));
    assert_eq!(eval("1u8 << 7u32").unwrap(), Value::U8(128));
}

#[test]
fn test_eval_binary_shr() {
    assert_eq!(eval("16 >> 2").unwrap(), Value::I64(4));
    assert_eq!(eval("128u8 >> 7u32").unwrap(), Value::U8(1));
}

// ═══════════════════════════════════════════════════════════════════════
// Type Mismatches
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_binary_type_mismatch() {
    let result = eval("1i32 + 1i64");
    assert!(matches!(
        result,
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
}

#[test]
fn test_eval_binary_invalid_types() {
    let result = eval("true + false");
    assert!(matches!(
        result,
        Err(EvalError::InvalidBinaryOperands { .. })
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// Parentheses and Precedence
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_parentheses() {
    assert_eq!(eval("(1 + 2) * 3").unwrap(), Value::I64(9));
    assert_eq!(eval("1 + (2 * 3)").unwrap(), Value::I64(7));
}

#[test]
fn test_eval_nested_expressions() {
    assert_eq!(eval("((1 + 2) * (3 + 4))").unwrap(), Value::I64(21));
}

// ═══════════════════════════════════════════════════════════════════════
// Complex Expressions with Variables
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_expression_with_vars() {
    let mut env = Environment::new();
    env.define("x", Value::I64(10));
    env.define("y", Value::I64(20));

    assert_eq!(eval_with_env("x + y", &mut env).unwrap(), Value::I64(30));
    assert_eq!(
        eval_with_env("x * y + 5", &mut env).unwrap(),
        Value::I64(205)
    );
    assert_eq!(eval_with_env("x < y", &mut env).unwrap(), Value::Bool(true));
}

// ═══════════════════════════════════════════════════════════════════════
// Interruption
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_interrupted() {
    let expr: syn::Expr = syn::parse_str("1 + 2").unwrap();
    let mut env = Environment::new();
    let ctx = EvalContext::default();

    // Set interrupt before evaluation
    ctx.interrupt();

    let result = expr.eval(&mut env, &ctx);
    assert!(matches!(result, Err(EvalError::Interrupted)));
}
