//! Expression evaluation

pub mod binary;
pub mod control;
pub mod if_expr;
pub mod literal;
pub mod loops;
pub mod match_expr;
pub mod path;
pub mod pattern;
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

            // Stage 1.5: Functions (not yet implemented)
            syn::Expr::Call(_) => Err(not_yet_implemented("function call", self)),
            syn::Expr::MethodCall(_) => Err(not_yet_implemented("method call", self)),
            syn::Expr::Closure(_) => Err(not_yet_implemented("closure", self)),
            syn::Expr::Return(_) => Err(not_yet_implemented("return", self)),

            // Stage 1.6: Blocks
            syn::Expr::Block(expr) => if_expr::eval_block(&expr.block, env, ctx),

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
pub use if_expr::eval_block;
pub use pattern::{apply_bindings, match_pattern};
