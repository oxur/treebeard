//! Rust language frontend for Treebeard
//!
//! This frontend parses Rust source code using `syn` and provides Rust-style
//! error and value formatting.

use crate::frontend::{LanguageFrontend, MacroEnvironment, MacroError, ParseError};
use crate::{EvalError, Value};

/// Rust language frontend.
///
/// Parses Rust source code into `syn` AST and provides Rust-style formatting
/// for errors and values.
///
/// # Example
///
/// ```
/// use treebeard::frontends::RustFrontend;
/// use treebeard::LanguageFrontend;
///
/// let frontend = RustFrontend::new();
/// let items = frontend.parse("fn main() { println!(\"Hello\"); }").unwrap();
/// assert_eq!(frontend.name(), "Rust");
/// assert_eq!(frontend.file_extension(), "rs");
/// ```
#[derive(Debug, Clone, Default)]
pub struct RustFrontend;

impl RustFrontend {
    /// Create a new Rust frontend.
    pub fn new() -> Self {
        Self
    }
}

impl LanguageFrontend for RustFrontend {
    fn parse(&self, source: &str) -> Result<Vec<syn::Item>, ParseError> {
        // Try parsing as a full file first
        if let Ok(file) = syn::parse_file(source) {
            return Ok(file.items);
        }

        // If that fails, try parsing as a single item
        if let Ok(item) = syn::parse_str::<syn::Item>(source) {
            return Ok(vec![item]);
        }

        // If both fail, return a parse error
        // Try to get a more specific error message
        match syn::parse_file(source) {
            Ok(_) => unreachable!(),
            Err(e) => Err(ParseError::new(format!("Rust syntax error: {}", e))),
        }
    }

    fn expand_macros(
        &self,
        items: Vec<syn::Item>,
        env: &MacroEnvironment,
    ) -> Result<(Vec<syn::Item>, MacroEnvironment), MacroError> {
        // Rust macros are already expanded by syn during parsing
        // No additional expansion needed
        Ok((items, env.clone()))
    }

    fn format_error(&self, error: &EvalError, _source: &str) -> String {
        // Format errors in Rust style
        match error {
            EvalError::UndefinedVariable { name, span: _ } => {
                format!("error: cannot find value `{}` in this scope", name)
            }
            EvalError::TypeError { message, span: _ } => {
                format!("error: type error: {}", message)
            }
            EvalError::ArityMismatch {
                expected,
                got,
                name,
                span: _,
            } => {
                format!(
                    "error: function `{}` takes {} argument{} but {} {} supplied",
                    name,
                    expected,
                    if *expected == 1 { "" } else { "s" },
                    got,
                    if *got == 1 { "was" } else { "were" }
                )
            }
            EvalError::IntegerOverflow { span: _ } => "error: integer overflow".to_string(),
            EvalError::DivisionByZero { span: _ } => "error: attempt to divide by zero".to_string(),
            _ => format!("error: {}", error),
        }
    }

    fn format_value(&self, value: &Value, depth: usize) -> String {
        format_value_rust(value, depth, 0)
    }

    fn name(&self) -> &str {
        "Rust"
    }

    fn file_extension(&self) -> &str {
        "rs"
    }
}

/// Format a value in Rust syntax.
fn format_value_rust(value: &Value, max_depth: usize, current_depth: usize) -> String {
    if current_depth >= max_depth && max_depth > 0 {
        return "...".to_string();
    }

    match value {
        Value::Unit => "()".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::I8(n) => format!("{}i8", n),
        Value::I16(n) => format!("{}i16", n),
        Value::I32(n) => format!("{}i32", n),
        Value::I64(n) => n.to_string(),
        Value::I128(n) => format!("{}i128", n),
        Value::Isize(n) => format!("{}isize", n),
        Value::U8(n) => format!("{}u8", n),
        Value::U16(n) => format!("{}u16", n),
        Value::U32(n) => format!("{}u32", n),
        Value::U64(n) => format!("{}u64", n),
        Value::U128(n) => format!("{}u128", n),
        Value::Usize(n) => format!("{}usize", n),
        Value::F32(f) => {
            if f.is_finite() {
                format!("{}f32", f)
            } else {
                format!("{:?}", f)
            }
        }
        Value::F64(f) => {
            if f.is_finite() {
                f.to_string()
            } else {
                format!("{:?}", f)
            }
        }
        Value::Char(c) => format!("'{}'", c.escape_default()),
        Value::String(s) => format!("\"{}\"", s.escape_default()),
        Value::Bytes(b) => format!("b{:?}", b.as_slice()),
        Value::Tuple(elements) => {
            if elements.is_empty() {
                "()".to_string()
            } else {
                let formatted: Vec<_> = elements
                    .iter()
                    .map(|v| format_value_rust(v, max_depth, current_depth + 1))
                    .collect();
                format!("({})", formatted.join(", "))
            }
        }
        Value::Array(elements) => {
            let formatted: Vec<_> = elements
                .iter()
                .take(10) // Limit to first 10 elements
                .map(|v| format_value_rust(v, max_depth, current_depth + 1))
                .collect();
            if elements.len() > 10 {
                format!("[{}, ...]", formatted.join(", "))
            } else {
                format!("[{}]", formatted.join(", "))
            }
        }
        Value::Vec(elements) => {
            let formatted: Vec<_> = elements
                .iter()
                .take(10) // Limit to first 10 elements
                .map(|v| format_value_rust(v, max_depth, current_depth + 1))
                .collect();
            if elements.len() > 10 {
                format!("vec![{}, ...]", formatted.join(", "))
            } else {
                format!("vec![{}]", formatted.join(", "))
            }
        }
        Value::Struct(s) => {
            if s.fields.is_empty() {
                s.type_name.clone()
            } else {
                let fields: Vec<_> = s
                    .fields
                    .iter()
                    .take(5)
                    .map(|(k, v)| {
                        format!(
                            "{}: {}",
                            k,
                            format_value_rust(v, max_depth, current_depth + 1)
                        )
                    })
                    .collect();
                if s.fields.len() > 5 {
                    format!("{} {{ {}, .. }}", s.type_name, fields.join(", "))
                } else {
                    format!("{} {{ {} }}", s.type_name, fields.join(", "))
                }
            }
        }
        Value::Enum(e) => match &e.data {
            crate::value::EnumData::Unit => e.variant.clone(),
            crate::value::EnumData::Tuple(values) => {
                let formatted: Vec<_> = values
                    .iter()
                    .map(|v| format_value_rust(v, max_depth, current_depth + 1))
                    .collect();
                format!("{}({})", e.variant, formatted.join(", "))
            }
            crate::value::EnumData::Struct(fields) => {
                let formatted: Vec<_> = fields
                    .iter()
                    .take(5)
                    .map(|(k, v)| {
                        format!(
                            "{}: {}",
                            k,
                            format_value_rust(v, max_depth, current_depth + 1)
                        )
                    })
                    .collect();
                if fields.len() > 5 {
                    format!("{} {{ {}, .. }}", e.variant, formatted.join(", "))
                } else {
                    format!("{} {{ {} }}", e.variant, formatted.join(", "))
                }
            }
        },
        Value::Option(opt) => match opt.as_ref() {
            Some(v) => format!(
                "Some({})",
                format_value_rust(v, max_depth, current_depth + 1)
            ),
            None => "None".to_string(),
        },
        Value::Result(res) => match res.as_ref() {
            Ok(v) => format!("Ok({})", format_value_rust(v, max_depth, current_depth + 1)),
            Err(e) => format!(
                "Err({})",
                format_value_rust(e, max_depth, current_depth + 1)
            ),
        },
        Value::HashMap(_) => "<HashMap>".to_string(),
        Value::Function(f) => format!("fn {}", f.name),
        Value::BuiltinFn(f) => format!("<builtin: {}>", f.name),
        Value::Closure(_) => "<closure>".to_string(),
        Value::CompiledFn(f) => format!("<compiled: {}>", f.name),
        Value::Ref(r) => format!(
            "&{}",
            format_value_rust(&r.value, max_depth, current_depth + 1)
        ),
        Value::RefMut(r) => {
            if let Ok(guard) = r.value.read() {
                format!(
                    "&mut {}",
                    format_value_rust(&guard, max_depth, current_depth + 1)
                )
            } else {
                "&mut <locked>".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_frontend_creation() {
        let frontend = RustFrontend::new();
        assert_eq!(frontend.name(), "Rust");
        assert_eq!(frontend.file_extension(), "rs");
    }

    #[test]
    fn test_parse_function() {
        let frontend = RustFrontend::new();
        let result = frontend.parse("fn main() {}");
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_const() {
        let frontend = RustFrontend::new();
        let result = frontend.parse("const X: i32 = 42;");
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_multiple_items() {
        let frontend = RustFrontend::new();
        let source = "const X: i32 = 42;\nfn main() {}";
        let result = frontend.parse(source);
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let frontend = RustFrontend::new();
        let result = frontend.parse("fn main(");
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_macros_noop() {
        let frontend = RustFrontend::new();
        let items = vec![];
        let env = MacroEnvironment::new();
        let result = frontend.expand_macros(items.clone(), &env);
        assert!(result.is_ok());
        let (expanded_items, _) = result.unwrap();
        assert_eq!(expanded_items.len(), 0);
    }

    #[test]
    fn test_format_value_primitives() {
        let frontend = RustFrontend::new();
        assert_eq!(frontend.format_value(&Value::Unit, 10), "()");
        assert_eq!(frontend.format_value(&Value::Bool(true), 10), "true");
        assert_eq!(frontend.format_value(&Value::I64(42), 10), "42");
        assert_eq!(frontend.format_value(&Value::U32(99), 10), "99u32");
        assert_eq!(frontend.format_value(&Value::F64(3.14), 10), "3.14");
    }

    #[test]
    fn test_format_value_string() {
        let frontend = RustFrontend::new();
        let value = Value::string("hello");
        assert_eq!(frontend.format_value(&value, 10), "\"hello\"");
    }

    #[test]
    fn test_format_value_tuple() {
        let frontend = RustFrontend::new();
        let value = Value::tuple(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
        assert_eq!(frontend.format_value(&value, 10), "(1, 2, 3)");
    }

    #[test]
    fn test_format_value_array() {
        let frontend = RustFrontend::new();
        let value = Value::array(vec![Value::I64(1), Value::I64(2)]);
        assert_eq!(frontend.format_value(&value, 10), "[1, 2]");
    }

    #[test]
    fn test_format_value_vec() {
        let frontend = RustFrontend::new();
        let value = Value::vec(vec![Value::I64(10), Value::I64(20)]);
        assert_eq!(frontend.format_value(&value, 10), "vec![10, 20]");
    }

    #[test]
    fn test_format_value_option_some() {
        let frontend = RustFrontend::new();
        let value = Value::some(Value::I64(42));
        assert_eq!(frontend.format_value(&value, 10), "Some(42)");
    }

    #[test]
    fn test_format_value_option_none() {
        let frontend = RustFrontend::new();
        let value = Value::none();
        assert_eq!(frontend.format_value(&value, 10), "None");
    }

    #[test]
    fn test_format_value_result_ok() {
        let frontend = RustFrontend::new();
        let value = Value::ok(Value::I64(42));
        assert_eq!(frontend.format_value(&value, 10), "Ok(42)");
    }

    #[test]
    fn test_format_value_result_err() {
        let frontend = RustFrontend::new();
        let value = Value::err(Value::string("error"));
        assert_eq!(frontend.format_value(&value, 10), "Err(\"error\")");
    }

    #[test]
    fn test_format_value_depth_limit() {
        let frontend = RustFrontend::new();
        let nested = Value::tuple(vec![Value::tuple(vec![Value::tuple(vec![Value::I64(42)])])]);
        let formatted = frontend.format_value(&nested, 2);
        assert!(formatted.contains("..."));
    }

    #[test]
    fn test_format_error_undefined_variable() {
        let frontend = RustFrontend::new();
        let error = EvalError::UndefinedVariable {
            name: "foo".to_string(),
            span: None,
        };
        let formatted = frontend.format_error(&error, "");
        assert!(formatted.contains("cannot find value `foo`"));
    }

    #[test]
    fn test_format_error_type_error() {
        let frontend = RustFrontend::new();
        let error = EvalError::TypeError {
            message: "expected i64, got String".to_string(),
            span: None,
        };
        let formatted = frontend.format_error(&error, "");
        assert!(formatted.contains("type error"));
        assert!(formatted.contains("expected i64, got String"));
    }

    #[test]
    fn test_format_error_arity_mismatch() {
        let frontend = RustFrontend::new();
        let error = EvalError::ArityMismatch {
            expected: 2,
            got: 3,
            name: "add".to_string(),
            span: None,
        };
        let formatted = frontend.format_error(&error, "");
        assert!(formatted.contains("function `add` takes 2 arguments"));
        assert!(formatted.contains("3 were supplied"));
    }
}
