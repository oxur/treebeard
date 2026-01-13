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
