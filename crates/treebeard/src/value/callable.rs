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
