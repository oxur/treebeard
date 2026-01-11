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
