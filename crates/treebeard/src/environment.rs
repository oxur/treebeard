//! Runtime environment managing variable and function bindings

mod frame;
mod prelude;

pub use frame::ScopeGuard;

use proc_macro2::Span;
use std::sync::Arc;

use crate::error::EnvironmentError;
use crate::value::{BuiltinFn, FunctionValue, Value};

/// A single variable or function binding.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The binding's name
    pub name: String,

    /// The bound value
    pub value: Value,

    /// Whether this binding is mutable
    pub mutable: bool,

    /// Where this binding was defined (for error messages)
    pub span: Option<Span>,
}

/// Binding mode for let statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingMode {
    /// Immutable binding: `let x = ...`
    Immutable,

    /// Mutable binding: `let mut x = ...`
    Mutable,

    /// Constant binding: `const X = ...`
    Constant,
}

/// The runtime environment managing variable and function bindings.
///
/// Uses a flat scope design with frame boundaries for efficient
/// scope entry/exit and cache-friendly lookups.
///
/// # Example
///
/// ```
/// use treebeard::{Environment, Value};
///
/// let mut env = Environment::new();
///
/// // Global scope
/// env.define("x", Value::I64(1));
///
/// // Enter a new scope
/// env.push_frame();
/// env.define("y", Value::I64(2));
/// env.define("x", Value::I64(10)); // Shadows outer x
///
/// assert_eq!(env.get("x"), Some(&Value::I64(10))); // Gets inner x
/// assert_eq!(env.get("y"), Some(&Value::I64(2)));
///
/// // Exit scope
/// env.pop_frame();
///
/// assert_eq!(env.get("x"), Some(&Value::I64(1))); // Back to outer x
/// assert_eq!(env.get("y"), None); // y is gone
/// ```
#[derive(Debug, Clone)]
pub struct Environment {
    /// All bindings in a flat array (most recent at end)
    bindings: Vec<Binding>,

    /// Frame boundaries (indices into bindings)
    /// Each entry marks where a scope begins
    frames: Vec<usize>,

    /// Current call depth (for recursion limiting)
    call_depth: usize,

    /// Maximum allowed call depth
    max_call_depth: usize,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    /// Create a new empty environment.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            frames: vec![0], // Start with one frame (global scope)
            call_depth: 0,
            max_call_depth: 1000,
        }
    }

    /// Create an environment with a custom call depth limit.
    pub fn with_max_call_depth(max_depth: usize) -> Self {
        Self {
            bindings: Vec::new(),
            frames: vec![0],
            call_depth: 0,
            max_call_depth: max_depth,
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Frame Management (Scope Entry/Exit)
    // ═══════════════════════════════════════════════════════════════════

    /// Enter a new scope (push a frame).
    ///
    /// All bindings defined after this call will be removed when
    /// `pop_frame()` is called.
    pub fn push_frame(&mut self) {
        self.frames.push(self.bindings.len());
    }

    /// Exit the current scope (pop a frame).
    ///
    /// Removes all bindings defined since the matching `push_frame()`.
    /// Does nothing if at the global scope (won't pop the last frame).
    pub fn pop_frame(&mut self) {
        // Never pop the global frame
        if self.frames.len() > 1 {
            if let Some(boundary) = self.frames.pop() {
                self.bindings.truncate(boundary);
            }
        }
    }

    /// Get the current scope depth (number of frames).
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Check if we're at global scope.
    pub fn is_global_scope(&self) -> bool {
        self.frames.len() == 1
    }

    // ═══════════════════════════════════════════════════════════════════
    // Call Depth Tracking (Stack Overflow Protection)
    // ═══════════════════════════════════════════════════════════════════

    /// Enter a function call. Returns error if max depth exceeded.
    pub fn enter_call(&mut self) -> Result<(), EnvironmentError> {
        if self.call_depth >= self.max_call_depth {
            return Err(EnvironmentError::StackOverflow {
                depth: self.call_depth,
                max: self.max_call_depth,
            });
        }
        self.call_depth += 1;
        Ok(())
    }

    /// Exit a function call.
    pub fn exit_call(&mut self) {
        self.call_depth = self.call_depth.saturating_sub(1);
    }

    /// Get current call depth.
    pub fn call_depth(&self) -> usize {
        self.call_depth
    }

    // ═══════════════════════════════════════════════════════════════════
    // Binding Definition
    // ═══════════════════════════════════════════════════════════════════

    /// Define a new immutable binding in the current scope.
    ///
    /// This always creates a new binding, even if a binding with the
    /// same name exists (shadowing).
    pub fn define(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.push(Binding {
            name: name.into(),
            value,
            mutable: false,
            span: None,
        });
    }

    /// Define a new binding with explicit mutability.
    pub fn define_with_mode(&mut self, name: impl Into<String>, value: Value, mode: BindingMode) {
        self.bindings.push(Binding {
            name: name.into(),
            value,
            mutable: mode == BindingMode::Mutable,
            span: None,
        });
    }

    /// Define a new binding with source span for error reporting.
    pub fn define_with_span(
        &mut self,
        name: impl Into<String>,
        value: Value,
        mutable: bool,
        span: Span,
    ) {
        self.bindings.push(Binding {
            name: name.into(),
            value,
            mutable,
            span: Some(span),
        });
    }

    /// Define a function in the environment.
    ///
    /// Convenience method that wraps the function in a Value.
    pub fn define_function(&mut self, func: FunctionValue) {
        let name = func.name.clone();
        // ALLOW: FunctionValue is Send + Sync (syn::Block is Send + Sync),
        // but clippy can't verify this automatically
        #[allow(clippy::arc_with_non_send_sync)]
        self.define(name, Value::Function(Arc::new(func)));
    }

    /// Register a built-in function.
    pub fn define_builtin(&mut self, builtin: BuiltinFn) {
        let name = builtin.name.clone();
        self.define(name, Value::BuiltinFn(builtin));
    }

    // ═══════════════════════════════════════════════════════════════════
    // Binding Lookup
    // ═══════════════════════════════════════════════════════════════════

    /// Look up a binding by name.
    ///
    /// Returns the most recent binding with the given name (shadowing),
    /// or `None` if not found.
    pub fn get(&self, name: &str) -> Option<&Value> {
        // Search backwards to find most recent binding
        self.bindings
            .iter()
            .rev()
            .find(|b| b.name == name)
            .map(|b| &b.value)
    }

    /// Look up a binding and return the full Binding struct.
    pub fn get_binding(&self, name: &str) -> Option<&Binding> {
        self.bindings.iter().rev().find(|b| b.name == name)
    }

    /// Look up a mutable reference to a binding's value.
    ///
    /// Returns `None` if the binding doesn't exist.
    /// Returns `Err` if the binding exists but is immutable.
    pub fn get_mut(&mut self, name: &str) -> Result<Option<&mut Value>, EnvironmentError> {
        // Find the index of the binding
        let idx = self
            .bindings
            .iter()
            .enumerate()
            .rev()
            .find(|(_, b)| b.name == name)
            .map(|(i, _)| i);

        match idx {
            Some(i) => {
                let binding = &self.bindings[i];
                if !binding.mutable {
                    return Err(EnvironmentError::ImmutableBinding {
                        name: name.to_string(),
                        span: binding.span,
                    });
                }
                Ok(Some(&mut self.bindings[i].value))
            }
            None => Ok(None),
        }
    }

    /// Check if a binding exists.
    pub fn contains(&self, name: &str) -> bool {
        self.bindings.iter().any(|b| b.name == name)
    }

    /// Check if a binding exists in the current (innermost) scope only.
    pub fn contains_in_current_scope(&self, name: &str) -> bool {
        let frame_start = *self.frames.last().unwrap_or(&0);
        self.bindings[frame_start..].iter().any(|b| b.name == name)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Assignment (Mutation)
    // ═══════════════════════════════════════════════════════════════════

    /// Assign a new value to an existing mutable binding.
    ///
    /// # Errors
    ///
    /// - `UndefinedVariable` if the binding doesn't exist
    /// - `ImmutableBinding` if the binding is not mutable
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), EnvironmentError> {
        // Find the binding (reverse search for shadowing)
        let idx = self
            .bindings
            .iter()
            .enumerate()
            .rev()
            .find(|(_, b)| b.name == name)
            .map(|(i, _)| i);

        match idx {
            Some(i) => {
                if !self.bindings[i].mutable {
                    return Err(EnvironmentError::ImmutableBinding {
                        name: name.to_string(),
                        span: self.bindings[i].span,
                    });
                }
                self.bindings[i].value = value;
                Ok(())
            }
            None => Err(EnvironmentError::UndefinedVariable {
                name: name.to_string(),
            }),
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Iteration and Inspection
    // ═══════════════════════════════════════════════════════════════════

    /// Iterate over all bindings (for debugging/REPL).
    pub fn iter(&self) -> impl Iterator<Item = &Binding> {
        self.bindings.iter()
    }

    /// Get all binding names in the current scope.
    pub fn names_in_current_scope(&self) -> Vec<&str> {
        let frame_start = *self.frames.last().unwrap_or(&0);
        self.bindings[frame_start..]
            .iter()
            .map(|b| b.name.as_str())
            .collect()
    }

    /// Get all binding names (for completion).
    pub fn all_names(&self) -> Vec<&str> {
        self.bindings.iter().map(|b| b.name.as_str()).collect()
    }

    /// Get the number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if the environment is empty.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Clear all bindings except built-ins (reset to initial state).
    pub fn clear(&mut self) {
        self.bindings.clear();
        self.frames = vec![0];
        self.call_depth = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::BuiltinFn;

    #[test]
    fn test_new_environment() {
        let env = Environment::new();
        assert_eq!(env.depth(), 1); // Global scope
        assert!(env.is_empty());
        assert_eq!(env.call_depth(), 0);
    }

    #[test]
    fn test_default_environment() {
        let env = Environment::default();
        assert_eq!(env.depth(), 1);
        assert!(env.is_empty());
    }

    #[test]
    fn test_with_max_call_depth() {
        let env = Environment::with_max_call_depth(500);
        assert_eq!(env.max_call_depth, 500);
        assert_eq!(env.depth(), 1);
    }

    #[test]
    fn test_define_and_get() {
        let mut env = Environment::new();
        env.define("x", Value::I64(42));

        assert_eq!(env.get("x"), Some(&Value::I64(42)));
        assert_eq!(env.get("y"), None);
    }

    #[test]
    fn test_push_pop_frame() {
        let mut env = Environment::new();
        assert_eq!(env.depth(), 1);

        env.push_frame();
        assert_eq!(env.depth(), 2);

        env.push_frame();
        assert_eq!(env.depth(), 3);

        env.pop_frame();
        assert_eq!(env.depth(), 2);

        env.pop_frame();
        assert_eq!(env.depth(), 1);
    }

    #[test]
    fn test_pop_frame_never_removes_global() {
        let mut env = Environment::new();
        assert_eq!(env.depth(), 1);

        env.pop_frame(); // Try to pop global
        assert_eq!(env.depth(), 1); // Should still be 1
    }

    #[test]
    fn test_is_global_scope() {
        let mut env = Environment::new();
        assert!(env.is_global_scope());

        env.push_frame();
        assert!(!env.is_global_scope());

        env.pop_frame();
        assert!(env.is_global_scope());
    }

    #[test]
    fn test_frame_scoping() {
        let mut env = Environment::new();

        env.define("x", Value::I64(1));
        assert_eq!(env.get("x"), Some(&Value::I64(1)));

        env.push_frame();
        env.define("y", Value::I64(2));
        assert_eq!(env.get("x"), Some(&Value::I64(1)));
        assert_eq!(env.get("y"), Some(&Value::I64(2)));

        env.pop_frame();
        assert_eq!(env.get("x"), Some(&Value::I64(1)));
        assert_eq!(env.get("y"), None); // y should be gone
    }

    #[test]
    fn test_shadowing() {
        let mut env = Environment::new();

        env.define("x", Value::I64(1));
        assert_eq!(env.get("x"), Some(&Value::I64(1)));

        env.push_frame();
        env.define("x", Value::I64(2)); // Shadow outer x
        assert_eq!(env.get("x"), Some(&Value::I64(2))); // See inner x

        env.pop_frame();
        assert_eq!(env.get("x"), Some(&Value::I64(1))); // Back to outer x
    }

    #[test]
    fn test_define_with_mode_immutable() {
        let mut env = Environment::new();
        env.define_with_mode("x", Value::I64(42), BindingMode::Immutable);

        let binding = env.get_binding("x").unwrap();
        assert!(!binding.mutable);
        assert_eq!(binding.value, Value::I64(42));
    }

    #[test]
    fn test_define_with_mode_mutable() {
        let mut env = Environment::new();
        env.define_with_mode("x", Value::I64(42), BindingMode::Mutable);

        let binding = env.get_binding("x").unwrap();
        assert!(binding.mutable);
        assert_eq!(binding.value, Value::I64(42));
    }

    #[test]
    fn test_define_with_mode_constant() {
        let mut env = Environment::new();
        env.define_with_mode("X", Value::I64(42), BindingMode::Constant);

        let binding = env.get_binding("X").unwrap();
        assert!(!binding.mutable); // Constants are immutable
        assert_eq!(binding.value, Value::I64(42));
    }

    #[test]
    fn test_define_with_span() {
        let mut env = Environment::new();
        let span = proc_macro2::Span::call_site();

        env.define_with_span("x", Value::I64(42), false, span);

        let binding = env.get_binding("x").unwrap();
        assert!(binding.span.is_some());
        assert!(!binding.mutable);
    }

    #[test]
    fn test_define_function() {
        let mut env = Environment::new();
        let block: syn::Block = syn::parse_quote!({});
        let func = FunctionValue::new("test_fn".to_string(), vec![], block);

        env.define_function(func);

        assert!(env.contains("test_fn"));
        match env.get("test_fn") {
            Some(Value::Function(_)) => {}
            _ => panic!("Expected function value"),
        }
    }

    #[test]
    fn test_define_builtin() {
        let mut env = Environment::new();
        let builtin = BuiltinFn {
            name: "test_builtin".to_string(),
            arity: 1,
            func: Arc::new(|_| Ok(Value::Unit)),
        };

        env.define_builtin(builtin);

        assert!(env.contains("test_builtin"));
        match env.get("test_builtin") {
            Some(Value::BuiltinFn(_)) => {}
            _ => panic!("Expected builtin function value"),
        }
    }

    #[test]
    fn test_get_binding() {
        let mut env = Environment::new();
        env.define("x", Value::I64(42));

        let binding = env.get_binding("x").unwrap();
        assert_eq!(binding.name, "x");
        assert_eq!(binding.value, Value::I64(42));

        assert!(env.get_binding("undefined").is_none());
    }

    #[test]
    fn test_get_mut_success() {
        let mut env = Environment::new();
        env.define_with_mode("x", Value::I64(42), BindingMode::Mutable);

        let val = env.get_mut("x").unwrap().unwrap();
        *val = Value::I64(100);

        assert_eq!(env.get("x"), Some(&Value::I64(100)));
    }

    #[test]
    fn test_get_mut_immutable_error() {
        let mut env = Environment::new();
        env.define("x", Value::I64(42)); // Immutable

        let result = env.get_mut("x");
        assert!(result.is_err());
        match result.unwrap_err() {
            EnvironmentError::ImmutableBinding { name, .. } => {
                assert_eq!(name, "x");
            }
            _ => panic!("Expected ImmutableBinding error"),
        }
    }

    #[test]
    fn test_get_mut_undefined() {
        let mut env = Environment::new();
        let result = env.get_mut("undefined").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_contains() {
        let mut env = Environment::new();
        env.define("x", Value::I64(42));

        assert!(env.contains("x"));
        assert!(!env.contains("y"));
    }

    #[test]
    fn test_contains_in_current_scope() {
        let mut env = Environment::new();
        env.define("x", Value::I64(1));

        assert!(env.contains_in_current_scope("x"));

        env.push_frame();
        assert!(!env.contains_in_current_scope("x")); // x is in outer scope

        env.define("y", Value::I64(2));
        assert!(env.contains_in_current_scope("y")); // y is in current scope
        assert!(env.contains("x")); // But x is still visible via contains()
    }

    #[test]
    fn test_assign_success() {
        let mut env = Environment::new();
        env.define_with_mode("x", Value::I64(42), BindingMode::Mutable);

        env.assign("x", Value::I64(100)).unwrap();
        assert_eq!(env.get("x"), Some(&Value::I64(100)));
    }

    #[test]
    fn test_assign_immutable_error() {
        let mut env = Environment::new();
        env.define("x", Value::I64(42)); // Immutable

        let result = env.assign("x", Value::I64(100));
        assert!(result.is_err());
        match result.unwrap_err() {
            EnvironmentError::ImmutableBinding { name, .. } => {
                assert_eq!(name, "x");
            }
            _ => panic!("Expected ImmutableBinding error"),
        }
    }

    #[test]
    fn test_assign_undefined_error() {
        let mut env = Environment::new();

        let result = env.assign("undefined", Value::I64(42));
        assert!(result.is_err());
        match result.unwrap_err() {
            EnvironmentError::UndefinedVariable { name } => {
                assert_eq!(name, "undefined");
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }

    #[test]
    fn test_enter_exit_call() {
        let mut env = Environment::new();
        assert_eq!(env.call_depth(), 0);

        env.enter_call().unwrap();
        assert_eq!(env.call_depth(), 1);

        env.enter_call().unwrap();
        assert_eq!(env.call_depth(), 2);

        env.exit_call();
        assert_eq!(env.call_depth(), 1);

        env.exit_call();
        assert_eq!(env.call_depth(), 0);
    }

    #[test]
    fn test_stack_overflow_protection() {
        let mut env = Environment::with_max_call_depth(3);

        env.enter_call().unwrap();
        env.enter_call().unwrap();
        env.enter_call().unwrap();

        let result = env.enter_call();
        assert!(result.is_err());
        match result.unwrap_err() {
            EnvironmentError::StackOverflow { depth, max } => {
                assert_eq!(depth, 3);
                assert_eq!(max, 3);
            }
            _ => panic!("Expected StackOverflow error"),
        }
    }

    #[test]
    fn test_exit_call_saturating() {
        let mut env = Environment::new();
        env.exit_call(); // Should not underflow
        assert_eq!(env.call_depth(), 0);
    }

    #[test]
    fn test_iter() {
        let mut env = Environment::new();
        env.define("x", Value::I64(1));
        env.define("y", Value::I64(2));

        let names: Vec<_> = env.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn test_names_in_current_scope() {
        let mut env = Environment::new();
        env.define("x", Value::I64(1));

        env.push_frame();
        env.define("y", Value::I64(2));
        env.define("z", Value::I64(3));

        let names = env.names_in_current_scope();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"y"));
        assert!(names.contains(&"z"));
        assert!(!names.contains(&"x")); // x is in outer scope
    }

    #[test]
    fn test_all_names() {
        let mut env = Environment::new();
        env.define("x", Value::I64(1));
        env.push_frame();
        env.define("y", Value::I64(2));

        let names = env.all_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"x"));
        assert!(names.contains(&"y"));
    }

    #[test]
    fn test_len() {
        let mut env = Environment::new();
        assert_eq!(env.len(), 0);

        env.define("x", Value::I64(1));
        assert_eq!(env.len(), 1);

        env.define("y", Value::I64(2));
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn test_is_empty() {
        let mut env = Environment::new();
        assert!(env.is_empty());

        env.define("x", Value::I64(1));
        assert!(!env.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut env = Environment::new();
        env.define("x", Value::I64(1));
        env.push_frame();
        env.define("y", Value::I64(2));
        env.enter_call().unwrap();

        assert!(!env.is_empty());
        assert_eq!(env.depth(), 2);
        assert_eq!(env.call_depth(), 1);

        env.clear();

        assert!(env.is_empty());
        assert_eq!(env.depth(), 1);
        assert_eq!(env.call_depth(), 0);
    }

    #[test]
    fn test_binding_mode_equality() {
        assert_eq!(BindingMode::Immutable, BindingMode::Immutable);
        assert_eq!(BindingMode::Mutable, BindingMode::Mutable);
        assert_eq!(BindingMode::Constant, BindingMode::Constant);
        assert_ne!(BindingMode::Immutable, BindingMode::Mutable);
    }

    #[test]
    fn test_multiple_shadowing_levels() {
        let mut env = Environment::new();

        env.define("x", Value::I64(1));
        env.push_frame();
        env.define("x", Value::I64(2));
        env.push_frame();
        env.define("x", Value::I64(3));

        assert_eq!(env.get("x"), Some(&Value::I64(3)));

        env.pop_frame();
        assert_eq!(env.get("x"), Some(&Value::I64(2)));

        env.pop_frame();
        assert_eq!(env.get("x"), Some(&Value::I64(1)));
    }

    #[test]
    fn test_assign_shadows_correctly() {
        let mut env = Environment::new();

        env.define_with_mode("x", Value::I64(1), BindingMode::Mutable);
        env.push_frame();
        env.define_with_mode("x", Value::I64(2), BindingMode::Mutable);

        // Assign should affect inner x
        env.assign("x", Value::I64(20)).unwrap();
        assert_eq!(env.get("x"), Some(&Value::I64(20)));

        env.pop_frame();
        // Outer x should be unchanged
        assert_eq!(env.get("x"), Some(&Value::I64(1)));
    }
}
