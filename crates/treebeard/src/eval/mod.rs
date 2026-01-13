//! Expression evaluation

pub mod array;
pub mod assign;
pub mod binary;
pub mod call;
pub mod control;
pub mod field;
pub mod function;
pub mod if_expr;
pub mod index;
pub mod item;
pub mod literal;
pub mod local;
pub mod loops;
pub mod match_expr;
pub mod path;
pub mod pattern;
pub mod range;
pub mod return_expr;
pub mod stmt;
pub mod struct_lit;
pub mod tuple;
pub mod unary;

use crate::{Environment, EvalContext, EvalError, Value};

/// Trait for evaluating AST nodes to values.
///
/// This is the core abstraction for the tree-walking interpreter.
/// Each `syn` expression type implements this trait.
pub trait Evaluate {
    /// Evaluate this AST node in the given environment.
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError>;
}

// ═══════════════════════════════════════════════════════════════════════
// Main Expression Dispatcher
// ═══════════════════════════════════════════════════════════════════════

impl Evaluate for syn::Expr {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Check for interruption before each expression
        if ctx.is_interrupted() {
            return Err(EvalError::Interrupted);
        }

        match self {
            // Stage 1.3: Basic expressions
            syn::Expr::Lit(expr) => expr.eval(env, ctx),
            syn::Expr::Path(expr) => expr.eval(env, ctx),
            syn::Expr::Unary(expr) => expr.eval(env, ctx),
            syn::Expr::Binary(expr) => expr.eval(env, ctx),

            // Stage 1.4: Control flow
            syn::Expr::If(expr) => expr.eval(env, ctx),
            syn::Expr::Match(expr) => expr.eval(env, ctx),
            syn::Expr::Loop(expr) => expr.eval(env, ctx),
            syn::Expr::While(expr) => expr.eval(env, ctx),
            syn::Expr::ForLoop(expr) => expr.eval(env, ctx),
            syn::Expr::Break(expr) => expr.eval(env, ctx),
            syn::Expr::Continue(expr) => expr.eval(env, ctx),

            // Stage 1.5: Functions
            syn::Expr::Call(expr) => expr.eval(env, ctx),
            syn::Expr::MethodCall(expr) => expr.eval(env, ctx),
            syn::Expr::Return(expr) => expr.eval(env, ctx),
            syn::Expr::Closure(_) => Err(not_yet_implemented("closure", self)),

            // Stage 1.6: Statements & Blocks
            syn::Expr::Block(expr) => stmt::eval_block(&expr.block, env, ctx),
            syn::Expr::Assign(expr) => assign::eval_assign(expr, env, ctx),
            syn::Expr::Index(expr) => index::eval_index(expr, env, ctx),
            syn::Expr::Field(expr) => field::eval_field(expr, env, ctx),
            syn::Expr::Tuple(expr) => tuple::eval_tuple(expr, env, ctx),
            syn::Expr::Array(expr) => array::eval_array(expr, env, ctx),
            syn::Expr::Repeat(expr) => array::eval_array_repeat(expr, env, ctx),
            syn::Expr::Struct(expr) => struct_lit::eval_struct(expr, env, ctx),
            syn::Expr::Range(expr) => range::eval_range(expr, env, ctx),

            // Parenthesized expressions - just unwrap
            syn::Expr::Paren(expr) => expr.expr.eval(env, ctx),

            // Group expressions (for precedence) - just unwrap
            syn::Expr::Group(expr) => expr.expr.eval(env, ctx),

            // Everything else
            _ => Err(EvalError::UnsupportedExpr {
                kind: expr_kind_name(self).to_string(),
                span: Some(expr_span(self)),
            }),
        }
    }
}

/// Get a human-readable name for an expression kind.
fn expr_kind_name(expr: &syn::Expr) -> &'static str {
    match expr {
        syn::Expr::Array(_) => "array",
        syn::Expr::Assign(_) => "assignment",
        syn::Expr::Async(_) => "async block",
        syn::Expr::Await(_) => "await",
        syn::Expr::Binary(_) => "binary operation",
        syn::Expr::Block(_) => "block",
        syn::Expr::Break(_) => "break",
        syn::Expr::Call(_) => "function call",
        syn::Expr::Cast(_) => "cast",
        syn::Expr::Closure(_) => "closure",
        syn::Expr::Const(_) => "const block",
        syn::Expr::Continue(_) => "continue",
        syn::Expr::Field(_) => "field access",
        syn::Expr::ForLoop(_) => "for loop",
        syn::Expr::Group(_) => "group",
        syn::Expr::If(_) => "if",
        syn::Expr::Index(_) => "index",
        syn::Expr::Infer(_) => "infer",
        syn::Expr::Let(_) => "let guard",
        syn::Expr::Lit(_) => "literal",
        syn::Expr::Loop(_) => "loop",
        syn::Expr::Macro(_) => "macro invocation",
        syn::Expr::Match(_) => "match",
        syn::Expr::MethodCall(_) => "method call",
        syn::Expr::Paren(_) => "parenthesized",
        syn::Expr::Path(_) => "path",
        syn::Expr::Range(_) => "range",
        syn::Expr::Reference(_) => "reference",
        syn::Expr::Repeat(_) => "repeat",
        syn::Expr::Return(_) => "return",
        syn::Expr::Struct(_) => "struct literal",
        syn::Expr::Try(_) => "try",
        syn::Expr::TryBlock(_) => "try block",
        syn::Expr::Tuple(_) => "tuple",
        syn::Expr::Unary(_) => "unary operation",
        syn::Expr::Unsafe(_) => "unsafe block",
        syn::Expr::Verbatim(_) => "verbatim",
        syn::Expr::While(_) => "while",
        syn::Expr::Yield(_) => "yield",
        _ => "unknown",
    }
}

/// Get the span of an expression.
fn expr_span(expr: &syn::Expr) -> proc_macro2::Span {
    use quote::ToTokens;
    expr.to_token_stream()
        .into_iter()
        .next()
        .map(|t| t.span())
        .unwrap_or_else(proc_macro2::Span::call_site)
}

/// Create a "not yet implemented" error.
fn not_yet_implemented(what: &str, expr: &syn::Expr) -> EvalError {
    EvalError::UnsupportedExpr {
        kind: format!("{} (not yet implemented)", what),
        span: Some(expr_span(expr)),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Convenience Functions
// ═══════════════════════════════════════════════════════════════════════

/// Evaluate an expression (convenience wrapper).
pub fn eval_expr(
    expr: &syn::Expr,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    expr.eval(env, ctx)
}

// Re-export for use by other modules
pub use control::ControlFlow;
pub use pattern::{apply_bindings, match_pattern};
pub use stmt::{eval_block, eval_block_stmts, eval_stmt};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_expr_literal() {
        let expr: syn::Expr = syn::parse_quote!(42);
        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = eval_expr(&expr, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_eval_expr_binary() {
        let expr: syn::Expr = syn::parse_quote!(1 + 2);
        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = eval_expr(&expr, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    #[test]
    fn test_eval_expr_paren() {
        let expr: syn::Expr = syn::parse_quote!((42));
        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = eval_expr(&expr, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_eval_expr_group() {
        let expr: syn::Expr = syn::parse_quote!({ 1 + 2 });
        let mut env = Environment::new();
        let ctx = EvalContext::default();
        let result = eval_expr(&expr, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    #[test]
    fn test_expr_kind_name() {
        let lit: syn::Expr = syn::parse_quote!(42);
        assert_eq!(expr_kind_name(&lit), "literal");

        let bin: syn::Expr = syn::parse_quote!(1 + 2);
        assert_eq!(expr_kind_name(&bin), "binary operation");

        let call: syn::Expr = syn::parse_quote!(foo());
        assert_eq!(expr_kind_name(&call), "function call");
    }

    #[test]
    fn test_not_yet_implemented() {
        let expr: syn::Expr = syn::parse_quote!(async {});
        let err = not_yet_implemented("async block", &expr);
        match err {
            EvalError::UnsupportedExpr { kind, .. } => {
                assert!(kind.contains("not yet implemented"));
            }
            _ => panic!("Expected UnsupportedExpr"),
        }
    }
}
