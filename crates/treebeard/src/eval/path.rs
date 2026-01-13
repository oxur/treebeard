//! Path evaluation (variable lookup)

use crate::{Environment, EvalContext, EvalError, Value};

use super::Evaluate;

impl Evaluate for syn::ExprPath {
    fn eval(&self, env: &mut Environment, _ctx: &EvalContext) -> Result<Value, EvalError> {
        // For now, we only support simple paths (single identifier)
        // Complex paths like `std::collections::HashMap` are not supported yet

        if self.path.segments.len() != 1 {
            return Err(EvalError::UnsupportedExpr {
                kind: format!("qualified path `{}`", path_to_string(&self.path)),
                span: Some(self.path.segments.first().unwrap().ident.span()),
            });
        }

        let segment = self.path.segments.first().unwrap();
        let name = segment.ident.to_string();

        // Check for path arguments (like `foo::<T>`)
        if !matches!(segment.arguments, syn::PathArguments::None) {
            return Err(EvalError::UnsupportedExpr {
                kind: format!("path with type arguments `{}`", name),
                span: Some(segment.ident.span()),
            });
        }

        // Look up in environment
        env.get(&name)
            .cloned()
            .ok_or_else(|| EvalError::UndefinedVariable {
                name,
                span: Some(segment.ident.span()),
            })
    }
}

/// Convert a syn::Path to a string for error messages.
pub fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_path_simple_variable_found() {
        let expr: syn::ExprPath = syn::parse_quote!(x);
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        // Define variable x
        env.define("x".to_string(), Value::I64(42));

        let result = expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_eval_path_simple_variable_undefined() {
        let expr: syn::ExprPath = syn::parse_quote!(undefined_var);
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::UndefinedVariable { name, .. } => {
                assert_eq!(name, "undefined_var");
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }

    #[test]
    fn test_eval_path_qualified_path_unsupported() {
        let expr: syn::ExprPath = syn::parse_quote!(std::io::Error);
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::UnsupportedExpr { kind, .. } => {
                assert!(kind.contains("qualified path"));
                assert!(kind.contains("std::io::Error"));
            }
            _ => panic!("Expected UnsupportedExpr error"),
        }
    }

    #[test]
    fn test_eval_path_two_segments_unsupported() {
        let expr: syn::ExprPath = syn::parse_quote!(module::function);
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::UnsupportedExpr { kind, .. } => {
                assert!(kind.contains("qualified path"));
                assert!(kind.contains("module::function"));
            }
            _ => panic!("Expected UnsupportedExpr error"),
        }
    }

    #[test]
    fn test_eval_path_with_type_arguments_unsupported() {
        let expr: syn::ExprPath = syn::parse_quote!(Vec::<i32>);
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::UnsupportedExpr { kind, .. } => {
                assert!(kind.contains("path with type arguments"));
                assert!(kind.contains("Vec"));
            }
            _ => panic!("Expected UnsupportedExpr error"),
        }
    }

    #[test]
    fn test_eval_path_different_value_types() {
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        // Test with different value types
        env.define("bool_var".to_string(), Value::Bool(true));
        env.define("string_var".to_string(), Value::string("hello"));
        env.define("unit_var".to_string(), Value::Unit);

        let bool_expr: syn::ExprPath = syn::parse_quote!(bool_var);
        assert_eq!(bool_expr.eval(&mut env, &ctx).unwrap(), Value::Bool(true));

        let string_expr: syn::ExprPath = syn::parse_quote!(string_var);
        assert_eq!(
            string_expr.eval(&mut env, &ctx).unwrap(),
            Value::string("hello")
        );

        let unit_expr: syn::ExprPath = syn::parse_quote!(unit_var);
        assert_eq!(unit_expr.eval(&mut env, &ctx).unwrap(), Value::Unit);
    }

    #[test]
    fn test_eval_path_shadowing() {
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        // Define variable in outer scope
        env.define("x".to_string(), Value::I64(1));

        // Push new frame and shadow
        env.push_frame();
        env.define("x".to_string(), Value::I64(2));

        let expr: syn::ExprPath = syn::parse_quote!(x);
        let result = expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(2)); // Should see inner scope value

        // Pop frame
        env.pop_frame();
        let result = expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(1)); // Should see outer scope value
    }

    #[test]
    fn test_path_to_string_single_segment() {
        let path: syn::Path = syn::parse_quote!(foo);
        assert_eq!(path_to_string(&path), "foo");
    }

    #[test]
    fn test_path_to_string_multiple_segments() {
        let path: syn::Path = syn::parse_quote!(std::io::Error);
        assert_eq!(path_to_string(&path), "std::io::Error");
    }

    #[test]
    fn test_path_to_string_with_type_arguments() {
        let path: syn::Path = syn::parse_quote!(Vec::<i32>);
        // path_to_string only extracts identifiers, not type arguments
        assert_eq!(path_to_string(&path), "Vec");
    }

    #[test]
    fn test_path_to_string_long_qualified_path() {
        let path: syn::Path = syn::parse_quote!(std::collections::hash_map::HashMap);
        assert_eq!(path_to_string(&path), "std::collections::hash_map::HashMap");
    }
}
