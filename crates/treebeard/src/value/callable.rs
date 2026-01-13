//! Callable value types: functions, closures, and builtins

use std::sync::Arc;

use super::Value;

/// Type alias for builtin function pointers to reduce complexity
pub type BuiltinFnPtr = Arc<dyn Fn(&[Value]) -> Result<Value, String> + Send + Sync>;

/// A user-defined function parsed from syn::ItemFn.
///
/// Stores the AST directly for interpretation.
#[derive(Debug, Clone)]
pub struct FunctionValue {
    /// Function name
    pub name: String,

    /// Parameter names (types are erased at runtime)
    pub params: Vec<String>,

    /// The function body (stored as syn AST)
    pub body: Arc<syn::Block>,

    /// Number of times this function has been called (for JIT heuristics)
    pub call_count: u64,
}

impl FunctionValue {
    /// Create a new function value
    pub fn new(name: String, params: Vec<String>, body: syn::Block) -> Self {
        Self {
            name,
            params,
            // ALLOW: syn::Block is Send + Sync (it's just AST data),
            // but clippy can't verify this automatically
            #[allow(clippy::arc_with_non_send_sync)]
            body: Arc::new(body),
            call_count: 0,
        }
    }
}

/// A closure with captured environment.
///
/// Closures capture variables from their defining scope.
/// This will be fully implemented in Phase 5.
#[derive(Debug, Clone)]
pub struct ClosureValue {
    /// Parameter names
    pub params: Vec<String>,

    /// The closure body
    pub body: Arc<syn::Expr>,

    /// Captured variables (name -> value)
    /// Uses Arc to allow sharing between closure copies
    pub captures: Arc<Vec<(String, Value)>>,
}

/// A built-in native function.
///
/// These are Rust functions exposed to the interpreter.
#[derive(Clone)]
pub struct BuiltinFn {
    /// Function name (for display/debugging)
    pub name: String,

    /// Arity (-1 for variadic)
    pub arity: i32,

    /// The actual function pointer
    /// Uses a trait object for flexibility
    pub func: BuiltinFnPtr,
}

impl std::fmt::Debug for BuiltinFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BuiltinFn({})", self.name)
    }
}

/// A compiled native function (loaded via dlopen).
///
/// This is the "escape hatch" to rustc for performance.
/// Will be implemented in Phase 6.
#[derive(Clone)]
pub struct CompiledFn {
    /// Function name
    pub name: String,

    /// Arity
    pub arity: usize,

    /// Path to the compiled library
    pub lib_path: std::path::PathBuf,

    /// Function pointer placeholder (requires unsafe for actual implementation)
    pub _marker: std::marker::PhantomData<()>,
}

impl std::fmt::Debug for CompiledFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompiledFn({} @ {:?})", self.name, self.lib_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_value_new() {
        let block: syn::Block = syn::parse_str("{ 42 }").unwrap();
        let func = FunctionValue::new(
            "test_fn".to_string(),
            vec!["x".to_string(), "y".to_string()],
            block,
        );
        assert_eq!(func.name, "test_fn");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.call_count, 0);
    }

    #[test]
    fn test_builtin_fn_debug() {
        let builtin = BuiltinFn {
            name: "test_builtin".to_string(),
            arity: 2,
            func: Arc::new(|_| Ok(Value::Unit)),
        };
        let debug_str = format!("{:?}", builtin);
        assert_eq!(debug_str, "BuiltinFn(test_builtin)");
    }

    #[test]
    fn test_compiled_fn_debug() {
        let compiled = CompiledFn {
            name: "test_compiled".to_string(),
            arity: 1,
            lib_path: std::path::PathBuf::from("/path/to/lib.so"),
            _marker: std::marker::PhantomData,
        };
        let debug_str = format!("{:?}", compiled);
        assert!(debug_str.contains("CompiledFn"));
        assert!(debug_str.contains("test_compiled"));
    }

    #[test]
    fn test_closure_value_structure() {
        let expr: syn::Expr = syn::parse_str("x + 1").unwrap();
        let closure = ClosureValue {
            params: vec!["x".to_string()],
            body: Arc::new(expr),
            captures: Arc::new(vec![("y".to_string(), Value::I64(42))]),
        };
        assert_eq!(closure.params.len(), 1);
        assert_eq!(closure.captures.len(), 1);
    }
}
