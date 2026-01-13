//! Function call evaluation

use std::sync::Arc;

use crate::eval::control::ControlFlow;
use crate::{BuiltinFn, ClosureValue, Environment, EvalContext, EvalError, FunctionValue, Value};

use super::Evaluate;

impl Evaluate for syn::ExprCall {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate the function expression
        let func_value = self.func.eval(env, ctx)?;

        // Evaluate arguments
        let args: Vec<Value> = self
            .args
            .iter()
            .map(|arg| arg.eval(env, ctx))
            .collect::<Result<Vec<_>, _>>()?;

        // Call the function
        call_value(func_value, args, env, ctx, None)
    }
}

impl Evaluate for syn::ExprMethodCall {
    fn eval(&self, env: &mut Environment, ctx: &EvalContext) -> Result<Value, EvalError> {
        // Evaluate the receiver
        let receiver = self.receiver.eval(env, ctx)?;

        // Evaluate arguments
        let mut args: Vec<Value> = vec![receiver];
        for arg in &self.args {
            args.push(arg.eval(env, ctx)?);
        }

        // Look up the method by name
        let method_name = self.method.to_string();

        // First, try built-in methods on the receiver type
        if let Some(result) = try_builtin_method(&method_name, &args)? {
            return Ok(result);
        }

        // Otherwise, look up as a regular function
        let func = env
            .get(&method_name)
            .cloned()
            .ok_or_else(|| EvalError::UndefinedVariable {
                name: method_name.clone(),
                span: Some(self.method.span()),
            })?;

        call_value(func, args, env, ctx, Some(self.method.span()))
    }
}

/// Call a Value as a function.
///
/// # Errors
///
/// Returns `TypeError` if the value is not callable.
/// Returns `ArityMismatch` if the argument count doesn't match.
pub fn call_value(
    func: Value,
    args: Vec<Value>,
    env: &mut Environment,
    ctx: &EvalContext,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    match func {
        Value::Function(f) => call_function(&f, args, env, ctx),
        Value::BuiltinFn(f) => call_builtin(&f, args, span),
        Value::Closure(c) => call_closure(&c, args, env, ctx),
        other => Err(EvalError::TypeError {
            message: format!(
                "expected function, found `{}`",
                crate::error::type_name(&other)
            ),
            span,
        }),
    }
}

/// Call a user-defined function.
fn call_function(
    func: &FunctionValue,
    args: Vec<Value>,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Check arity
    if args.len() != func.params.len() {
        return Err(EvalError::ArityMismatch {
            expected: func.params.len(),
            got: args.len(),
            name: func.name.clone(),
            span: None,
        });
    }

    // Track call depth (stack overflow protection)
    env.enter_call()?;

    // Create new scope for function body
    env.push_frame();

    // Bind parameters to arguments
    for (param, arg) in func.params.iter().zip(args.into_iter()) {
        env.define(param.clone(), arg);
    }

    // Evaluate the function body
    let result = eval_function_body(&func.body, env, ctx);

    // Clean up
    env.pop_frame();
    env.exit_call();

    // Handle return control flow
    match result {
        Ok(value) => Ok(value),
        Err(EvalError::ControlFlow(ControlFlow::Return { value })) => Ok(value),
        Err(e) => Err(e),
    }
}

/// Call a built-in function.
fn call_builtin(
    func: &BuiltinFn,
    args: Vec<Value>,
    span: Option<proc_macro2::Span>,
) -> Result<Value, EvalError> {
    // Check arity (if not variadic)
    if func.arity >= 0 && args.len() != func.arity as usize {
        return Err(EvalError::ArityMismatch {
            expected: func.arity as usize,
            got: args.len(),
            name: func.name.clone(),
            span,
        });
    }

    // Call the native function
    (func.func)(&args).map_err(|e| EvalError::BuiltinError {
        name: func.name.clone(),
        message: e,
        span,
    })
}

/// Call a closure.
fn call_closure(
    closure: &ClosureValue,
    args: Vec<Value>,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Check arity
    if args.len() != closure.params.len() {
        return Err(EvalError::ArityMismatch {
            expected: closure.params.len(),
            got: args.len(),
            name: "<closure>".to_string(),
            span: None,
        });
    }

    // Track call depth
    env.enter_call()?;

    // Create new scope
    env.push_frame();

    // Bind captured variables first
    for (name, value) in closure.captures.iter() {
        env.define(name.clone(), value.clone());
    }

    // Bind parameters
    for (param, arg) in closure.params.iter().zip(args.into_iter()) {
        env.define(param.clone(), arg);
    }

    // Evaluate the closure body
    let result = closure.body.eval(env, ctx);

    // Clean up
    env.pop_frame();
    env.exit_call();

    // Handle return
    match result {
        Ok(value) => Ok(value),
        Err(EvalError::ControlFlow(ControlFlow::Return { value })) => Ok(value),
        Err(e) => Err(e),
    }
}

/// Evaluate a function body (block).
fn eval_function_body(
    body: &syn::Block,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let mut last_value = Value::Unit;

    for stmt in &body.stmts {
        last_value = eval_stmt_in_function(stmt, env, ctx)?;
    }

    Ok(last_value)
}

/// Evaluate a statement within a function body.
fn eval_stmt_in_function(
    stmt: &syn::Stmt,
    env: &mut Environment,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    match stmt {
        syn::Stmt::Expr(expr, None) => {
            // Expression without semicolon - its value is the result
            expr.eval(env, ctx)
        }
        syn::Stmt::Expr(expr, Some(_)) => {
            // Expression with semicolon - evaluate for side effects
            expr.eval(env, ctx)?;
            Ok(Value::Unit)
        }
        syn::Stmt::Local(local) => {
            // Let binding - delegate to local module
            super::local::eval_local(local, env, ctx)?;
            Ok(Value::Unit)
        }
        syn::Stmt::Item(item) => {
            // Item in function (nested fn, etc.)
            super::item::eval_item(item, env, ctx)?;
            Ok(Value::Unit)
        }
        syn::Stmt::Macro(_) => Err(EvalError::UnsupportedExpr {
            kind: "macro statement".to_string(),
            span: None,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Built-in Methods
// ═══════════════════════════════════════════════════════════════════════

/// Try to call a built-in method on a value.
///
/// Returns `Ok(Some(value))` if the method was handled as a built-in.
/// Returns `Ok(None)` if no built-in method matched.
/// Returns `Err` if the built-in method failed.
fn try_builtin_method(method: &str, args: &[Value]) -> Result<Option<Value>, EvalError> {
    if args.is_empty() {
        return Ok(None);
    }

    let receiver = &args[0];
    let method_args = &args[1..];

    match (receiver, method) {
        // String methods
        (Value::String(s), "len") if method_args.is_empty() => Ok(Some(Value::Usize(s.len()))),
        (Value::String(s), "is_empty") if method_args.is_empty() => {
            Ok(Some(Value::Bool(s.is_empty())))
        }
        (Value::String(s), "to_uppercase") if method_args.is_empty() => {
            Ok(Some(Value::string(s.to_uppercase())))
        }
        (Value::String(s), "to_lowercase") if method_args.is_empty() => {
            Ok(Some(Value::string(s.to_lowercase())))
        }
        (Value::String(s), "trim") if method_args.is_empty() => Ok(Some(Value::string(s.trim()))),
        (Value::String(s), "chars") if method_args.is_empty() => {
            Ok(Some(Value::vec(s.chars().map(Value::Char).collect())))
        }
        (Value::String(s), "contains") if method_args.len() == 1 => match &method_args[0] {
            Value::String(needle) => Ok(Some(Value::Bool(s.contains(needle.as_str())))),
            Value::Char(c) => Ok(Some(Value::Bool(s.contains(*c)))),
            _ => Ok(None),
        },
        (Value::String(s), "starts_with") if method_args.len() == 1 => {
            if let Value::String(prefix) = &method_args[0] {
                Ok(Some(Value::Bool(s.starts_with(prefix.as_str()))))
            } else {
                Ok(None)
            }
        }
        (Value::String(s), "ends_with") if method_args.len() == 1 => {
            if let Value::String(suffix) = &method_args[0] {
                Ok(Some(Value::Bool(s.ends_with(suffix.as_str()))))
            } else {
                Ok(None)
            }
        }

        // Vec methods
        (Value::Vec(v), "len") if method_args.is_empty() => Ok(Some(Value::Usize(v.len()))),
        (Value::Vec(v), "is_empty") if method_args.is_empty() => {
            Ok(Some(Value::Bool(v.is_empty())))
        }
        (Value::Vec(v), "first") if method_args.is_empty() => {
            Ok(Some(Value::Option(Arc::new(v.first().cloned()))))
        }
        (Value::Vec(v), "last") if method_args.is_empty() => {
            Ok(Some(Value::Option(Arc::new(v.last().cloned()))))
        }
        (Value::Vec(v), "get") if method_args.len() == 1 => {
            // Convert index to usize
            let idx_opt = match &method_args[0] {
                Value::Usize(n) => Some(*n),
                Value::I64(n) if *n >= 0 => Some(*n as usize),
                Value::I32(n) if *n >= 0 => Some(*n as usize),
                _ => None,
            };

            if let Some(idx) = idx_opt {
                Ok(Some(Value::Option(Arc::new(v.get(idx).cloned()))))
            } else {
                Ok(None)
            }
        }
        (Value::Vec(v), "contains") if method_args.len() == 1 => {
            Ok(Some(Value::Bool(v.contains(&method_args[0]))))
        }

        // Array methods (same as Vec)
        (Value::Array(v), "len") if method_args.is_empty() => Ok(Some(Value::Usize(v.len()))),
        (Value::Array(v), "is_empty") if method_args.is_empty() => {
            Ok(Some(Value::Bool(v.is_empty())))
        }
        (Value::Array(v), "first") if method_args.is_empty() => {
            Ok(Some(Value::Option(Arc::new(v.first().cloned()))))
        }
        (Value::Array(v), "last") if method_args.is_empty() => {
            Ok(Some(Value::Option(Arc::new(v.last().cloned()))))
        }

        // Option methods
        (Value::Option(opt), "is_some") if method_args.is_empty() => {
            Ok(Some(Value::Bool(opt.is_some())))
        }
        (Value::Option(opt), "is_none") if method_args.is_empty() => {
            Ok(Some(Value::Bool(opt.is_none())))
        }
        (Value::Option(opt), "unwrap") if method_args.is_empty() => match opt.as_ref() {
            Some(v) => Ok(Some(v.clone())),
            None => Err(EvalError::BuiltinError {
                name: "unwrap".to_string(),
                message: "called `Option::unwrap()` on a `None` value".to_string(),
                span: None,
            }),
        },
        (Value::Option(opt), "unwrap_or") if method_args.len() == 1 => {
            Ok(Some(match opt.as_ref() {
                Some(v) => v.clone(),
                None => method_args[0].clone(),
            }))
        }

        // Result methods
        (Value::Result(res), "is_ok") if method_args.is_empty() => {
            Ok(Some(Value::Bool(res.is_ok())))
        }
        (Value::Result(res), "is_err") if method_args.is_empty() => {
            Ok(Some(Value::Bool(res.is_err())))
        }
        (Value::Result(res), "unwrap") if method_args.is_empty() => match res.as_ref() {
            Ok(v) => Ok(Some(v.clone())),
            Err(e) => Err(EvalError::BuiltinError {
                name: "unwrap".to_string(),
                message: format!("called `Result::unwrap()` on an `Err` value: {:?}", e),
                span: None,
            }),
        },
        (Value::Result(res), "unwrap_err") if method_args.is_empty() => match res.as_ref() {
            Ok(v) => Err(EvalError::BuiltinError {
                name: "unwrap_err".to_string(),
                message: format!("called `Result::unwrap_err()` on an `Ok` value: {:?}", v),
                span: None,
            }),
            Err(e) => Ok(Some(e.clone())),
        },

        // Clone (works on most values)
        (_, "clone") if method_args.is_empty() => Ok(Some(receiver.clone())),

        // No built-in method found
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_len() {
        let result = try_builtin_method("len", &[Value::string("hello")]).unwrap();
        assert_eq!(result, Some(Value::Usize(5)));
    }

    #[test]
    fn test_string_is_empty() {
        let result = try_builtin_method("is_empty", &[Value::string("")]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let result = try_builtin_method("is_empty", &[Value::string("hi")]).unwrap();
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn test_string_to_uppercase() {
        let result = try_builtin_method("to_uppercase", &[Value::string("hello")]).unwrap();
        assert_eq!(result, Some(Value::string("HELLO")));
    }

    #[test]
    fn test_string_to_lowercase() {
        let result = try_builtin_method("to_lowercase", &[Value::string("HELLO")]).unwrap();
        assert_eq!(result, Some(Value::string("hello")));
    }

    #[test]
    fn test_string_trim() {
        let result = try_builtin_method("trim", &[Value::string("  hello  ")]).unwrap();
        assert_eq!(result, Some(Value::string("hello")));
    }

    #[test]
    fn test_string_contains() {
        let result =
            try_builtin_method("contains", &[Value::string("hello"), Value::string("ell")])
                .unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let result =
            try_builtin_method("contains", &[Value::string("hello"), Value::Char('e')]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));
    }

    #[test]
    fn test_vec_len() {
        let v = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
        let result = try_builtin_method("len", &[v]).unwrap();
        assert_eq!(result, Some(Value::Usize(3)));
    }

    #[test]
    fn test_vec_first_last() {
        let v = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);

        let result = try_builtin_method("first", &[v.clone()]).unwrap();
        assert!(matches!(result, Some(Value::Option(_))));

        let result = try_builtin_method("last", &[v]).unwrap();
        assert!(matches!(result, Some(Value::Option(_))));
    }

    #[test]
    fn test_option_is_some_is_none() {
        let some_val = Value::Option(Arc::new(Some(Value::I64(42))));
        let none_val = Value::Option(Arc::new(None));

        let result = try_builtin_method("is_some", &[some_val.clone()]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let result = try_builtin_method("is_none", &[none_val.clone()]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));
    }

    #[test]
    fn test_option_unwrap() {
        let some_val = Value::Option(Arc::new(Some(Value::I64(42))));
        let result = try_builtin_method("unwrap", &[some_val]).unwrap();
        assert_eq!(result, Some(Value::I64(42)));

        let none_val = Value::Option(Arc::new(None));
        let result = try_builtin_method("unwrap", &[none_val]);
        assert!(result.is_err());
    }

    #[test]
    fn test_result_is_ok_is_err() {
        let ok_val = Value::Result(Arc::new(Ok(Value::I64(42))));
        let err_val = Value::Result(Arc::new(Err(Value::string("error"))));

        let result = try_builtin_method("is_ok", &[ok_val]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let result = try_builtin_method("is_err", &[err_val]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));
    }

    #[test]
    fn test_clone() {
        let val = Value::I64(42);
        let result = try_builtin_method("clone", &[val.clone()]).unwrap();
        assert_eq!(result, Some(val));
    }

    #[test]
    fn test_string_chars() {
        let result = try_builtin_method("chars", &[Value::string("hi")]).unwrap();
        if let Some(Value::Vec(chars)) = result {
            assert_eq!(chars.len(), 2);
            assert_eq!(chars[0], Value::Char('h'));
            assert_eq!(chars[1], Value::Char('i'));
        } else {
            panic!("Expected Some(Vec)");
        }
    }

    #[test]
    fn test_string_starts_with() {
        let result = try_builtin_method(
            "starts_with",
            &[Value::string("hello"), Value::string("hel")],
        )
        .unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let result = try_builtin_method(
            "starts_with",
            &[Value::string("hello"), Value::string("bye")],
        )
        .unwrap();
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn test_string_ends_with() {
        let result =
            try_builtin_method("ends_with", &[Value::string("hello"), Value::string("lo")])
                .unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let result =
            try_builtin_method("ends_with", &[Value::string("hello"), Value::string("x")]).unwrap();
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn test_vec_is_empty() {
        let empty = Value::vec(vec![]);
        let result = try_builtin_method("is_empty", &[empty]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let non_empty = Value::vec(vec![Value::I64(1)]);
        let result = try_builtin_method("is_empty", &[non_empty]).unwrap();
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn test_vec_get() {
        let v = Value::vec(vec![Value::I64(10), Value::I64(20), Value::I64(30)]);

        // Valid index
        let result = try_builtin_method("get", &[v.clone(), Value::Usize(1)]).unwrap();
        if let Some(Value::Option(opt)) = result {
            assert!(opt.is_some());
            if let Some(val) = opt.as_ref() {
                assert_eq!(val, &Value::I64(20));
            }
        } else {
            panic!("Expected Some(Option)");
        }

        // Out of bounds
        let result = try_builtin_method("get", &[v, Value::Usize(10)]).unwrap();
        if let Some(Value::Option(opt)) = result {
            assert!(opt.is_none());
        } else {
            panic!("Expected Some(Option)");
        }
    }

    #[test]
    fn test_vec_contains() {
        let v = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);

        let result = try_builtin_method("contains", &[v.clone(), Value::I64(2)]).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));

        let result = try_builtin_method("contains", &[v, Value::I64(99)]).unwrap();
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn test_array_methods() {
        let arr = Value::array(vec![Value::I64(1), Value::I64(2)]);

        let result = try_builtin_method("len", &[arr.clone()]).unwrap();
        assert_eq!(result, Some(Value::Usize(2)));

        let result = try_builtin_method("is_empty", &[arr.clone()]).unwrap();
        assert_eq!(result, Some(Value::Bool(false)));

        let result = try_builtin_method("first", &[arr.clone()]).unwrap();
        assert!(matches!(result, Some(Value::Option(_))));

        let result = try_builtin_method("last", &[arr]).unwrap();
        assert!(matches!(result, Some(Value::Option(_))));
    }

    #[test]
    fn test_option_unwrap_or() {
        let some_val = Value::Option(Arc::new(Some(Value::I64(42))));
        let result = try_builtin_method("unwrap_or", &[some_val, Value::I64(0)]).unwrap();
        assert_eq!(result, Some(Value::I64(42)));

        let none_val = Value::Option(Arc::new(None));
        let result = try_builtin_method("unwrap_or", &[none_val, Value::I64(0)]).unwrap();
        assert_eq!(result, Some(Value::I64(0)));
    }

    #[test]
    fn test_result_unwrap() {
        let ok_val = Value::Result(Arc::new(Ok(Value::I64(42))));
        let result = try_builtin_method("unwrap", &[ok_val]).unwrap();
        assert_eq!(result, Some(Value::I64(42)));

        let err_val = Value::Result(Arc::new(Err(Value::string("error"))));
        let result = try_builtin_method("unwrap", &[err_val]);
        assert!(result.is_err());
    }

    #[test]
    fn test_result_unwrap_err() {
        let err_val = Value::Result(Arc::new(Err(Value::string("error"))));
        let result = try_builtin_method("unwrap_err", &[err_val]).unwrap();
        assert_eq!(result, Some(Value::string("error")));

        let ok_val = Value::Result(Arc::new(Ok(Value::I64(42))));
        let result = try_builtin_method("unwrap_err", &[ok_val]);
        assert!(result.is_err());
    }

    #[test]
    fn test_call_builtin_arity_mismatch() {
        let builtin = BuiltinFn {
            name: "test".to_string(),
            arity: 2,
            func: Arc::new(|args| {
                let sum = match (&args[0], &args[1]) {
                    (Value::I64(a), Value::I64(b)) => a + b,
                    _ => return Err("type error".to_string()),
                };
                Ok(Value::I64(sum))
            }),
        };

        // Wrong arity
        let result = call_builtin(&builtin, vec![Value::I64(1)], None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::ArityMismatch { .. }
        ));
    }

    #[test]
    fn test_call_builtin_variadic() {
        let builtin = BuiltinFn {
            name: "sum".to_string(),
            arity: -1, // variadic
            func: Arc::new(|args| {
                let mut sum = 0i64;
                for arg in args {
                    if let Value::I64(n) = arg {
                        sum += n;
                    }
                }
                Ok(Value::I64(sum))
            }),
        };

        let result = call_builtin(
            &builtin,
            vec![Value::I64(1), Value::I64(2), Value::I64(3)],
            None,
        );
        assert_eq!(result.unwrap(), Value::I64(6));
    }

    #[test]
    fn test_call_builtin_error() {
        let builtin = BuiltinFn {
            name: "fail".to_string(),
            arity: 0,
            func: Arc::new(|_| Err("intentional error".to_string())),
        };

        let result = call_builtin(&builtin, vec![], None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::BuiltinError { .. }
        ));
    }

    #[test]
    fn test_call_function_basic() {
        let body: syn::Block = syn::parse_str("{ 42 }").unwrap();
        let func = FunctionValue::new("test".to_string(), vec![], body);

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_function(&func, vec![], &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_call_function_with_params() {
        let body: syn::Block = syn::parse_str("{ x }").unwrap();
        let func = FunctionValue::new("identity".to_string(), vec!["x".to_string()], body);

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_function(&func, vec![Value::I64(99)], &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(99));
    }

    #[test]
    fn test_call_function_arity_mismatch() {
        let body: syn::Block = syn::parse_str("{ 42 }").unwrap();
        let func = FunctionValue::new("test".to_string(), vec!["x".to_string()], body);

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_function(&func, vec![], &mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::ArityMismatch { .. }
        ));
    }

    #[test]
    fn test_call_closure_basic() {
        let body: syn::Expr = syn::parse_str("42").unwrap();
        let closure = ClosureValue {
            params: vec![],
            body: Arc::new(body),
            captures: Arc::new(vec![]),
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_closure(&closure, vec![], &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_call_closure_with_capture() {
        let body: syn::Expr = syn::parse_str("x").unwrap();
        let closure = ClosureValue {
            params: vec![],
            body: Arc::new(body),
            captures: Arc::new(vec![("x".to_string(), Value::I64(100))]),
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_closure(&closure, vec![], &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(100));
    }

    #[test]
    fn test_call_closure_arity_mismatch() {
        let body: syn::Expr = syn::parse_str("x").unwrap();
        let closure = ClosureValue {
            params: vec!["x".to_string()],
            body: Arc::new(body),
            captures: Arc::new(vec![]),
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_closure(&closure, vec![], &mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::ArityMismatch { .. }
        ));
    }

    #[test]
    fn test_call_value_not_callable() {
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_value(Value::I64(42), vec![], &mut env, &ctx, None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EvalError::TypeError { .. }));
    }

    #[test]
    fn test_try_builtin_method_no_args() {
        let result = try_builtin_method("len", &[]).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_builtin_method_unknown_method() {
        let result = try_builtin_method("unknown_method", &[Value::I64(42)]).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_expr_call_eval() {
        // Define a function in environment and call it
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        // First define a function: fn add(a, b) { a + b }
        let fn_source = "fn add(a: i64, b: i64) -> i64 { a + b }";
        let item: syn::Item = syn::parse_str(fn_source).unwrap();
        super::super::item::eval_item(&item, &mut env, &ctx).unwrap();

        // Now call it: add(10, 20)
        let call_expr: syn::Expr = syn::parse_str("add(10, 20)").unwrap();
        let result = call_expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(30));
    }

    #[test]
    fn test_expr_method_call_eval() {
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        // Test a built-in method call: "hello".len()
        let method_expr: syn::Expr = syn::parse_str(r#""hello".len()"#).unwrap();
        let result = method_expr.eval(&mut env, &ctx).unwrap();
        assert_eq!(result, Value::Usize(5));
    }

    #[test]
    fn test_expr_method_call_undefined() {
        let mut env = Environment::new();
        let ctx = EvalContext::default();

        // Test calling an undefined method: 42.foo()
        let method_expr: syn::Expr = syn::parse_str("42.foo()").unwrap();
        let result = method_expr.eval(&mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::UndefinedVariable { .. }
        ));
    }

    #[test]
    fn test_eval_function_body_with_statements() {
        let body: syn::Block = syn::parse_str(
            r#"{
            let x = 10;
            let y = 20;
            x + y
        }"#,
        )
        .unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_function_body(&body, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(30));
    }

    #[test]
    fn test_eval_stmt_in_function_macro_unsupported() {
        let stmt: syn::Stmt = syn::parse_str("println!(\"test\");").unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_stmt_in_function(&stmt, &mut env, &ctx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvalError::UnsupportedExpr { .. }
        ));
    }

    #[test]
    fn test_eval_stmt_expr_with_semicolon() {
        let stmt: syn::Stmt = syn::parse_str("42;").unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_stmt_in_function(&stmt, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_eval_stmt_expr_without_semicolon() {
        // Parse from a block to get a statement without semicolon
        let block: syn::Block = syn::parse_str("{ 42 }").unwrap();
        let stmt = &block.stmts[0];

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_stmt_in_function(stmt, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_eval_stmt_local() {
        let stmt: syn::Stmt = syn::parse_str("let x = 100;").unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_stmt_in_function(&stmt, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
        assert_eq!(env.get("x"), Some(&Value::I64(100)));
    }

    #[test]
    fn test_eval_stmt_item() {
        let stmt: syn::Stmt = syn::parse_str("fn inner() -> i64 { 99 }").unwrap();

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = eval_stmt_in_function(&stmt, &mut env, &ctx).unwrap();
        assert_eq!(result, Value::Unit);
        assert!(env.get("inner").is_some());
    }

    #[test]
    fn test_call_function_with_return() {
        let body: syn::Block = syn::parse_str("{ return 42; }").unwrap();
        let func = FunctionValue::new("test".to_string(), vec![], body);

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_function(&func, vec![], &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_call_closure_with_param() {
        let body: syn::Expr = syn::parse_str("x + y").unwrap();
        let closure = ClosureValue {
            params: vec!["x".to_string()],
            body: Arc::new(body),
            captures: Arc::new(vec![("y".to_string(), Value::I64(5))]),
        };

        let mut env = Environment::new();
        let ctx = EvalContext::default();

        let result = call_closure(&closure, vec![Value::I64(10)], &mut env, &ctx).unwrap();
        assert_eq!(result, Value::I64(15));
    }
}
