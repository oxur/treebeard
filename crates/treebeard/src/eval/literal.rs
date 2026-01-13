//! Literal evaluation

use crate::{Environment, EvalContext, EvalError, Value};

use super::Evaluate;

impl Evaluate for syn::ExprLit {
    fn eval(&self, _env: &mut Environment, _ctx: &EvalContext) -> Result<Value, EvalError> {
        eval_lit(&self.lit)
    }
}

/// Evaluate a literal to a Value.
pub fn eval_lit(lit: &syn::Lit) -> Result<Value, EvalError> {
    match lit {
        syn::Lit::Str(s) => Ok(Value::string(s.value())),

        syn::Lit::ByteStr(bs) => Ok(Value::bytes(bs.value())),

        syn::Lit::CStr(_) => Err(EvalError::UnsupportedLiteral {
            kind: "C string literal".to_string(),
            span: Some(lit.span()),
        }),

        syn::Lit::Byte(b) => Ok(Value::U8(b.value())),

        syn::Lit::Char(c) => Ok(Value::Char(c.value())),

        syn::Lit::Int(i) => eval_int_literal(i),

        syn::Lit::Float(f) => eval_float_literal(f),

        syn::Lit::Bool(b) => Ok(Value::Bool(b.value())),

        syn::Lit::Verbatim(_) => Err(EvalError::UnsupportedLiteral {
            kind: "verbatim literal".to_string(),
            span: Some(lit.span()),
        }),

        _ => Err(EvalError::UnsupportedLiteral {
            kind: "unknown literal".to_string(),
            span: Some(lit.span()),
        }),
    }
}

/// Evaluate an integer literal, respecting suffixes.
fn eval_int_literal(lit: &syn::LitInt) -> Result<Value, EvalError> {
    let suffix = lit.suffix();
    let span = Some(lit.span());

    // Parse based on suffix
    match suffix {
        "i8" => lit
            .base10_parse::<i8>()
            .map(Value::I8)
            .map_err(|_| overflow_error(span)),
        "i16" => lit
            .base10_parse::<i16>()
            .map(Value::I16)
            .map_err(|_| overflow_error(span)),
        "i32" => lit
            .base10_parse::<i32>()
            .map(Value::I32)
            .map_err(|_| overflow_error(span)),
        "i64" => lit
            .base10_parse::<i64>()
            .map(Value::I64)
            .map_err(|_| overflow_error(span)),
        "i128" => lit
            .base10_parse::<i128>()
            .map(Value::I128)
            .map_err(|_| overflow_error(span)),
        "isize" => lit
            .base10_parse::<isize>()
            .map(Value::Isize)
            .map_err(|_| overflow_error(span)),
        "u8" => lit
            .base10_parse::<u8>()
            .map(Value::U8)
            .map_err(|_| overflow_error(span)),
        "u16" => lit
            .base10_parse::<u16>()
            .map(Value::U16)
            .map_err(|_| overflow_error(span)),
        "u32" => lit
            .base10_parse::<u32>()
            .map(Value::U32)
            .map_err(|_| overflow_error(span)),
        "u64" => lit
            .base10_parse::<u64>()
            .map(Value::U64)
            .map_err(|_| overflow_error(span)),
        "u128" => lit
            .base10_parse::<u128>()
            .map(Value::U128)
            .map_err(|_| overflow_error(span)),
        "usize" => lit
            .base10_parse::<usize>()
            .map(Value::Usize)
            .map_err(|_| overflow_error(span)),
        "" => {
            // No suffix - default to i64 (like Rust's type inference default for integers)
            lit.base10_parse::<i64>()
                .map(Value::I64)
                .map_err(|_| overflow_error(span))
        }
        other => Err(EvalError::UnsupportedLiteral {
            kind: format!("integer with suffix `{}`", other),
            span,
        }),
    }
}

/// Evaluate a float literal, respecting suffixes.
fn eval_float_literal(lit: &syn::LitFloat) -> Result<Value, EvalError> {
    let suffix = lit.suffix();
    let span = Some(lit.span());

    match suffix {
        "f32" => lit
            .base10_parse::<f32>()
            .map(Value::F32)
            .map_err(|e| EvalError::TypeError {
                message: format!("invalid f32 literal: {}", e),
                span,
            }),
        "f64" | "" => {
            // No suffix defaults to f64
            lit.base10_parse::<f64>()
                .map(Value::F64)
                .map_err(|e| EvalError::TypeError {
                    message: format!("invalid f64 literal: {}", e),
                    span,
                })
        }
        other => Err(EvalError::UnsupportedLiteral {
            kind: format!("float with suffix `{}`", other),
            span,
        }),
    }
}

fn overflow_error(span: Option<proc_macro2::Span>) -> EvalError {
    EvalError::IntegerOverflow { span }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_literal() {
        let lit: syn::Lit = syn::parse_str(r#""hello""#).unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::string("hello"));
    }

    #[test]
    fn test_byte_string_literal() {
        let lit: syn::Lit = syn::parse_str(r#"b"hello""#).unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::bytes(b"hello".to_vec()));
    }

    #[test]
    fn test_byte_literal() {
        let lit: syn::Lit = syn::parse_str("b'A'").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::U8(65));
    }

    #[test]
    fn test_char_literal() {
        let lit: syn::Lit = syn::parse_str("'x'").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::Char('x'));
    }

    #[test]
    fn test_bool_literal_true() {
        let lit: syn::Lit = syn::parse_str("true").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_bool_literal_false() {
        let lit: syn::Lit = syn::parse_str("false").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_int_literal_no_suffix() {
        let lit: syn::Lit = syn::parse_str("42").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_int_literal_i8() {
        let lit: syn::Lit = syn::parse_str("42i8").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::I8(42));
    }

    #[test]
    fn test_int_literal_i16() {
        let lit: syn::Lit = syn::parse_str("42i16").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::I16(42));
    }

    #[test]
    fn test_int_literal_i32() {
        let lit: syn::Lit = syn::parse_str("42i32").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::I32(42));
    }

    #[test]
    fn test_int_literal_i64() {
        let lit: syn::Lit = syn::parse_str("42i64").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_int_literal_i128() {
        let lit: syn::Lit = syn::parse_str("42i128").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::I128(42));
    }

    #[test]
    fn test_int_literal_isize() {
        let lit: syn::Lit = syn::parse_str("42isize").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::Isize(42));
    }

    #[test]
    fn test_int_literal_u8() {
        let lit: syn::Lit = syn::parse_str("42u8").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::U8(42));
    }

    #[test]
    fn test_int_literal_u16() {
        let lit: syn::Lit = syn::parse_str("42u16").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::U16(42));
    }

    #[test]
    fn test_int_literal_u32() {
        let lit: syn::Lit = syn::parse_str("42u32").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::U32(42));
    }

    #[test]
    fn test_int_literal_u64() {
        let lit: syn::Lit = syn::parse_str("42u64").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::U64(42));
    }

    #[test]
    fn test_int_literal_u128() {
        let lit: syn::Lit = syn::parse_str("42u128").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::U128(42));
    }

    #[test]
    fn test_int_literal_usize() {
        let lit: syn::Lit = syn::parse_str("42usize").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::Usize(42));
    }

    #[test]
    fn test_float_literal_no_suffix() {
        let lit: syn::Lit = syn::parse_str("3.14").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::F64(3.14));
    }

    #[test]
    fn test_float_literal_f32() {
        let lit: syn::Lit = syn::parse_str("3.14f32").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::F32(3.14));
    }

    #[test]
    fn test_float_literal_f64() {
        let lit: syn::Lit = syn::parse_str("3.14f64").unwrap();
        let result = eval_lit(&lit).unwrap();
        assert_eq!(result, Value::F64(3.14));
    }
}
