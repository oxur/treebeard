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
