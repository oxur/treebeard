//! Oxur language frontend for Treebeard
//!
//! This frontend parses Oxur S-expressions using oxur-ast and provides Oxur-style
//! error and value formatting.
//!
//! # Pipeline
//!
//! ```text
//! Oxur Source → S-Exp → Oxur AST → Rust Source → syn AST → Treebeard
//! ```
//!
//! This leverages Oxur's existing AST Bridge infrastructure (95% complete).

use crate::frontend::{LanguageFrontend, MacroEnvironment, MacroError, ParseError, SourceLocation};
use crate::{EvalError, Value};

/// Oxur language frontend.
///
/// Parses Oxur S-expressions into `syn` AST via the oxur-ast bridge and provides
/// Oxur-style formatting for errors and values.
///
/// # Example
///
/// ```no_run
/// use treebeard::frontends::OxurFrontend;
/// use treebeard::LanguageFrontend;
///
/// let frontend = OxurFrontend::new();
/// let items = frontend.parse("(defn main [] (println \"Hello from Oxur!\"))").unwrap();
/// assert_eq!(frontend.name(), "Oxur");
/// assert_eq!(frontend.file_extension(), "oxr");
/// ```
#[derive(Debug, Clone, Default)]
pub struct OxurFrontend;

impl OxurFrontend {
    /// Create a new Oxur frontend.
    pub fn new() -> Self {
        Self
    }

    /// Parse Oxur S-expressions to syn AST via the oxur-ast bridge.
    ///
    /// This performs the full pipeline:
    /// 1. Parse S-expressions
    /// 2. Build Oxur AST
    /// 3. Generate Rust code
    /// 4. Parse to syn AST
    fn parse_oxur_to_syn(&self, source: &str) -> Result<Vec<syn::Item>, ParseError> {
        // Step 1: Parse Oxur S-expressions
        let sexp = oxur_ast::sexp::Parser::parse_str(source).map_err(|e| {
            ParseError::new(format!("Oxur parse error: {}", e))
                .with_location(SourceLocation::new("<input>", 1, 1))
        })?;

        // Step 2: Build Oxur AST
        let mut builder = oxur_ast::builder::AstBuilder::new();
        let crate_ast = builder.build_crate(&sexp).map_err(|e| {
            ParseError::new(format!("Oxur AST build error: {}", e))
                .with_location(SourceLocation::new("<input>", 1, 1))
        })?;

        // Step 3: Generate Rust code from Oxur AST
        let rust_code = oxur_ast::gen_rs::generate_rust(&crate_ast).map_err(|e| {
            ParseError::new(format!("Rust code generation error: {}", e))
                .with_location(SourceLocation::new("<input>", 1, 1))
        })?;

        // Step 4: Parse Rust code to syn AST
        let syn_file = syn::parse_file(&rust_code).map_err(|e| {
            ParseError::new(format!("Rust syntax error: {}", e))
                .with_location(SourceLocation::new("<input>", 1, 1))
        })?;

        Ok(syn_file.items)
    }
}

impl LanguageFrontend for OxurFrontend {
    fn parse(&self, source: &str) -> Result<Vec<syn::Item>, ParseError> {
        self.parse_oxur_to_syn(source)
    }

    fn expand_macros(
        &self,
        items: Vec<syn::Item>,
        env: &MacroEnvironment,
    ) -> Result<(Vec<syn::Item>, MacroEnvironment), MacroError> {
        // Macro expansion will be implemented in Phase 3
        // For now, just pass through unchanged
        Ok((items, env.clone()))
    }

    fn format_error(&self, error: &EvalError, _source: &str) -> String {
        // Format errors in Oxur/Lisp style
        match error {
            EvalError::UndefinedVariable { name, span: _ } => {
                format!("error: undefined variable: {}", name)
            }
            EvalError::TypeError { message, span: _ } => {
                format!("error: type-error: {}", message)
            }
            EvalError::ArityMismatch {
                expected,
                got,
                name,
                span: _,
            } => {
                format!(
                    "error: arity-mismatch: function '{}' expects {} argument{}, got {}",
                    name,
                    expected,
                    if *expected == 1 { "" } else { "s" },
                    got
                )
            }
            EvalError::IntegerOverflow { span: _ } => "error: integer-overflow".to_string(),
            EvalError::DivisionByZero { span: _ } => "error: division-by-zero".to_string(),
            _ => format!("error: {}", error),
        }
    }

    fn format_value(&self, value: &Value, depth: usize) -> String {
        format_value_oxur(value, depth, 0)
    }

    fn name(&self) -> &str {
        "Oxur"
    }

    fn file_extension(&self) -> &str {
        "oxr"
    }
}

/// Format a value in Oxur/Lisp syntax.
fn format_value_oxur(value: &Value, max_depth: usize, current_depth: usize) -> String {
    if current_depth >= max_depth && max_depth > 0 {
        return "...".to_string();
    }

    match value {
        Value::Unit => "nil".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::I8(n) => n.to_string(),
        Value::I16(n) => n.to_string(),
        Value::I32(n) => n.to_string(),
        Value::I64(n) => n.to_string(),
        Value::I128(n) => n.to_string(),
        Value::Isize(n) => n.to_string(),
        Value::U8(n) => n.to_string(),
        Value::U16(n) => n.to_string(),
        Value::U32(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::U128(n) => n.to_string(),
        Value::Usize(n) => n.to_string(),
        Value::F32(f) => {
            if f.is_finite() {
                f.to_string()
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
        Value::Char(c) => format!("\\{}", c),
        Value::String(s) => format!("\"{}\"", s.escape_default()),
        Value::Bytes(b) => format!("#bytes[{}]", b.len()),
        Value::Tuple(elements) => {
            if elements.is_empty() {
                "()".to_string()
            } else {
                let formatted: Vec<_> = elements
                    .iter()
                    .map(|v| format_value_oxur(v, max_depth, current_depth + 1))
                    .collect();
                format!("(tuple {})", formatted.join(" "))
            }
        }
        Value::Array(elements) | Value::Vec(elements) => {
            let formatted: Vec<_> = elements
                .iter()
                .take(20) // Limit display
                .map(|v| format_value_oxur(v, max_depth, current_depth + 1))
                .collect();
            if elements.len() > 20 {
                format!("[{} ...]", formatted.join(" "))
            } else {
                format!("[{}]", formatted.join(" "))
            }
        }
        Value::Struct(s) => {
            if s.fields.is_empty() {
                format!("(struct {})", s.type_name)
            } else {
                let fields: Vec<_> = s
                    .fields
                    .iter()
                    .take(5)
                    .map(|(k, v)| {
                        format!(
                            ":{} {}",
                            k,
                            format_value_oxur(v, max_depth, current_depth + 1)
                        )
                    })
                    .collect();
                if s.fields.len() > 5 {
                    format!("(struct {} {} ...)", s.type_name, fields.join(" "))
                } else {
                    format!("(struct {} {})", s.type_name, fields.join(" "))
                }
            }
        }
        Value::Enum(e) => match &e.data {
            crate::value::EnumData::Unit => format!(":{}", e.variant),
            crate::value::EnumData::Tuple(values) => {
                let formatted: Vec<_> = values
                    .iter()
                    .map(|v| format_value_oxur(v, max_depth, current_depth + 1))
                    .collect();
                format!("({} {})", e.variant, formatted.join(" "))
            }
            crate::value::EnumData::Struct(fields) => {
                let formatted: Vec<_> = fields
                    .iter()
                    .take(5)
                    .map(|(k, v)| {
                        format!(
                            ":{} {}",
                            k,
                            format_value_oxur(v, max_depth, current_depth + 1)
                        )
                    })
                    .collect();
                if fields.len() > 5 {
                    format!("({} {} ...)", e.variant, formatted.join(" "))
                } else {
                    format!("({} {})", e.variant, formatted.join(" "))
                }
            }
        },
        Value::Option(opt) => match opt.as_ref() {
            Some(v) => format!(
                "(some {})",
                format_value_oxur(v, max_depth, current_depth + 1)
            ),
            None => "none".to_string(),
        },
        Value::Result(res) => match res.as_ref() {
            Ok(v) => format!(
                "(ok {})",
                format_value_oxur(v, max_depth, current_depth + 1)
            ),
            Err(e) => format!(
                "(err {})",
                format_value_oxur(e, max_depth, current_depth + 1)
            ),
        },
        Value::HashMap(_) => "#<hash-map>".to_string(),
        Value::Function(f) => format!("#<function:{}>", f.name),
        Value::BuiltinFn(f) => format!("#<builtin:{}>", f.name),
        Value::Closure(_) => "#<closure>".to_string(),
        Value::CompiledFn(f) => format!("#<compiled:{}>", f.name),
        Value::Ref(r) => format!(
            "(ref {})",
            format_value_oxur(&r.value, max_depth, current_depth + 1)
        ),
        Value::RefMut(r) => {
            if let Ok(guard) = r.value.read() {
                format!(
                    "(ref-mut {})",
                    format_value_oxur(&guard, max_depth, current_depth + 1)
                )
            } else {
                "(ref-mut #<locked>)".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oxur_frontend_creation() {
        let frontend = OxurFrontend::new();
        assert_eq!(frontend.name(), "Oxur");
        assert_eq!(frontend.file_extension(), "oxr");
    }

    #[test]
    fn test_format_value_primitives() {
        let frontend = OxurFrontend::new();
        assert_eq!(frontend.format_value(&Value::Unit, 10), "nil");
        assert_eq!(frontend.format_value(&Value::Bool(true), 10), "true");
        assert_eq!(frontend.format_value(&Value::Bool(false), 10), "false");
        assert_eq!(frontend.format_value(&Value::I64(42), 10), "42");
        assert_eq!(frontend.format_value(&Value::F64(3.14), 10), "3.14");
    }

    #[test]
    fn test_format_value_string() {
        let frontend = OxurFrontend::new();
        let value = Value::string("hello");
        assert_eq!(frontend.format_value(&value, 10), "\"hello\"");
    }

    #[test]
    fn test_format_value_vec() {
        let frontend = OxurFrontend::new();
        let value = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
        assert_eq!(frontend.format_value(&value, 10), "[1 2 3]");
    }

    #[test]
    fn test_format_value_option_some() {
        let frontend = OxurFrontend::new();
        let value = Value::some(Value::I64(42));
        assert_eq!(frontend.format_value(&value, 10), "(some 42)");
    }

    #[test]
    fn test_format_value_option_none() {
        let frontend = OxurFrontend::new();
        let value = Value::none();
        assert_eq!(frontend.format_value(&value, 10), "none");
    }

    #[test]
    fn test_format_value_result_ok() {
        let frontend = OxurFrontend::new();
        let value = Value::ok(Value::I64(42));
        assert_eq!(frontend.format_value(&value, 10), "(ok 42)");
    }

    #[test]
    fn test_format_value_result_err() {
        let frontend = OxurFrontend::new();
        let value = Value::err(Value::string("error"));
        assert_eq!(frontend.format_value(&value, 10), "(err \"error\")");
    }

    #[test]
    fn test_format_error_undefined_variable() {
        let frontend = OxurFrontend::new();
        let error = EvalError::UndefinedVariable {
            name: "foo".to_string(),
            span: None,
        };
        let formatted = frontend.format_error(&error, "");
        assert!(formatted.contains("undefined variable"));
        assert!(formatted.contains("foo"));
    }

    #[test]
    fn test_format_error_type_error() {
        let frontend = OxurFrontend::new();
        let error = EvalError::TypeError {
            message: "expected number, got string".to_string(),
            span: None,
        };
        let formatted = frontend.format_error(&error, "");
        assert!(formatted.contains("type-error"));
        assert!(formatted.contains("expected number, got string"));
    }

    #[test]
    fn test_format_error_arity_mismatch() {
        let frontend = OxurFrontend::new();
        let error = EvalError::ArityMismatch {
            expected: 2,
            got: 3,
            name: "add".to_string(),
            span: None,
        };
        let formatted = frontend.format_error(&error, "");
        assert!(formatted.contains("arity-mismatch"));
        assert!(formatted.contains("add"));
        assert!(formatted.contains("2"));
        assert!(formatted.contains("3"));
    }

    #[test]
    fn test_expand_macros_passthrough() {
        let frontend = OxurFrontend::new();
        let items = vec![];
        let env = MacroEnvironment::new();
        let result = frontend.expand_macros(items.clone(), &env);
        assert!(result.is_ok());
        let (expanded_items, _) = result.unwrap();
        assert_eq!(expanded_items.len(), 0);
    }
}
