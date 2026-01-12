# Stage 1.2: Environment

**Phase:** 1 - Core Evaluator
**Stage:** 1.2
**Prerequisites:** Stage 1.1 (Value Representation)
**Estimated effort:** 2-3 days

---

## Objective

Implement the `Environment` struct that manages variable and function bindings with lexical scoping. This provides the foundation for variable lookup, definition, and scope management during evaluation.

---

## Overview

The Environment uses a **flat scope with frame boundaries** design:

- **Flat array** of bindings for cache-friendly access
- **Frame stack** tracking scope boundaries (indices into the bindings array)
- **Reverse lookup** finds most recent binding (shadowing)
- **Frame pop** truncates bindings to restore previous scope

This pattern is proven in Rhai and provides good performance for interpreter use cases.

```
Bindings:  [a=1, b=2, c=3, d=4, e=5]
Frames:    [0, 2, 4]
            ^  ^  ^
            |  |  └── Frame 2 starts at index 4 (contains: e)
            |  └───── Frame 1 starts at index 2 (contains: c, d)
            └──────── Frame 0 starts at index 0 (contains: a, b)

pop_frame() → truncate bindings to index 4 → [a=1, b=2, c=3, d=4]
```

---

## File Structure

Add to the existing `treebeard` crate:

```
treebeard/src/
├── lib.rs              # Add: pub mod environment;
├── value.rs            # From Stage 1.1
├── environment.rs      # ← This stage (main file)
├── environment/
│   ├── mod.rs          # Module re-exports
│   ├── binding.rs      # Binding struct
│   ├── frame.rs        # Frame management
│   └── lookup.rs       # Lookup algorithms
└── error.rs            # Add: environment errors
```

---

## Core Types

### src/environment.rs

```rust
use std::sync::Arc;
use proc_macro2::Span;

use crate::value::{Value, FunctionValue, BuiltinFn};

/// The runtime environment managing variable and function bindings.
///
/// Uses a flat scope design with frame boundaries for efficient
/// scope entry/exit and cache-friendly lookups.
///
/// # Example
///
/// ```
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
    pub fn define_with_mode(
        &mut self,
        name: impl Into<String>,
        value: Value,
        mode: BindingMode,
    ) {
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
        self.bindings[frame_start..]
            .iter()
            .any(|b| b.name == name)
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
```

---

## Environment Errors

### Add to src/error.rs

```rust
use proc_macro2::Span;
use thiserror::Error;

/// Errors that can occur during environment operations.
#[derive(Error, Debug, Clone)]
pub enum EnvironmentError {
    /// Attempted to access an undefined variable.
    #[error("undefined variable `{name}`")]
    UndefinedVariable { name: String },

    /// Attempted to mutate an immutable binding.
    #[error("cannot assign to immutable binding `{name}`")]
    ImmutableBinding { name: String, span: Option<Span> },

    /// Call stack overflow (too much recursion).
    #[error("stack overflow: call depth {depth} exceeds maximum {max}")]
    StackOverflow { depth: usize, max: usize },

    /// Attempted to redefine a constant.
    #[error("cannot redefine constant `{name}`")]
    ConstantRedefinition { name: String },
}
```

---

## Scope Guard (RAII Pattern)

A helper for automatic scope cleanup:

### src/environment/frame.rs

```rust
use super::Environment;

/// RAII guard that automatically pops a frame when dropped.
///
/// # Example
///
/// ```
/// let mut env = Environment::new();
/// env.define("x", Value::I64(1));
///
/// {
///     let _guard = env.scope_guard();
///     env.define("y", Value::I64(2));
///     // y is visible here
/// }
/// // _guard dropped, frame popped, y is gone
/// ```
pub struct ScopeGuard<'a> {
    env: &'a mut Environment,
}

impl Environment {
    /// Create a scope guard that pushes a frame now and pops it on drop.
    pub fn scope_guard(&mut self) -> ScopeGuard<'_> {
        self.push_frame();
        ScopeGuard { env: self }
    }
}

impl<'a> Drop for ScopeGuard<'a> {
    fn drop(&mut self) {
        self.env.pop_frame();
    }
}

impl<'a> std::ops::Deref for ScopeGuard<'a> {
    type Target = Environment;

    fn deref(&self) -> &Self::Target {
        self.env
    }
}

impl<'a> std::ops::DerefMut for ScopeGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.env
    }
}
```

---

## Prelude (Standard Bindings)

A module for setting up the standard environment with built-in functions:

### src/environment/prelude.rs

```rust
use std::sync::Arc;
use crate::value::{Value, BuiltinFn};
use super::Environment;

impl Environment {
    /// Create an environment with standard built-in functions.
    pub fn with_prelude() -> Self {
        let mut env = Self::new();
        env.load_prelude();
        env
    }

    /// Load the standard prelude into this environment.
    pub fn load_prelude(&mut self) {
        // Printing
        self.define_builtin(BuiltinFn {
            name: "print".to_string(),
            arity: -1, // Variadic
            func: Arc::new(builtin_print),
        });

        self.define_builtin(BuiltinFn {
            name: "println".to_string(),
            arity: -1,
            func: Arc::new(builtin_println),
        });

        // Type inspection
        self.define_builtin(BuiltinFn {
            name: "type_of".to_string(),
            arity: 1,
            func: Arc::new(builtin_type_of),
        });

        // Debug representation
        self.define_builtin(BuiltinFn {
            name: "dbg".to_string(),
            arity: 1,
            func: Arc::new(builtin_dbg),
        });

        // Assertions
        self.define_builtin(BuiltinFn {
            name: "assert".to_string(),
            arity: 1,
            func: Arc::new(builtin_assert),
        });

        self.define_builtin(BuiltinFn {
            name: "assert_eq".to_string(),
            arity: 2,
            func: Arc::new(builtin_assert_eq),
        });

        // Panic
        self.define_builtin(BuiltinFn {
            name: "panic".to_string(),
            arity: -1,
            func: Arc::new(builtin_panic),
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Built-in Function Implementations
// ═══════════════════════════════════════════════════════════════════════

fn builtin_print(args: &[Value]) -> Result<Value, String> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg);
    }
    Ok(Value::Unit)
}

fn builtin_println(args: &[Value]) -> Result<Value, String> {
    builtin_print(args)?;
    println!();
    Ok(Value::Unit)
}

fn builtin_type_of(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("type_of expects 1 argument, got {}", args.len()));
    }

    let type_name = match &args[0] {
        Value::Unit => "()",
        Value::Bool(_) => "bool",
        Value::Char(_) => "char",
        Value::I8(_) => "i8",
        Value::I16(_) => "i16",
        Value::I32(_) => "i32",
        Value::I64(_) => "i64",
        Value::I128(_) => "i128",
        Value::Isize(_) => "isize",
        Value::U8(_) => "u8",
        Value::U16(_) => "u16",
        Value::U32(_) => "u32",
        Value::U64(_) => "u64",
        Value::U128(_) => "u128",
        Value::Usize(_) => "usize",
        Value::F32(_) => "f32",
        Value::F64(_) => "f64",
        Value::String(_) => "String",
        Value::Bytes(_) => "Vec<u8>",
        Value::Vec(_) => "Vec",
        Value::Tuple(_) => "tuple",
        Value::Array(_) => "array",
        Value::Struct(s) => return Ok(Value::string(&s.type_name)),
        Value::Enum(e) => return Ok(Value::string(&e.type_name)),
        Value::HashMap(_) => "HashMap",
        Value::Option(_) => "Option",
        Value::Result(_) => "Result",
        Value::Function(_) => "fn",
        Value::Closure(_) => "closure",
        Value::BuiltinFn(_) => "builtin_fn",
        Value::CompiledFn(_) => "compiled_fn",
        Value::Ref(_) => "ref",
        Value::RefMut(_) => "ref_mut",
    };

    Ok(Value::string(type_name))
}

fn builtin_dbg(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("dbg expects 1 argument, got {}", args.len()));
    }

    eprintln!("[dbg] {:?}", args[0]);
    Ok(args[0].clone())
}

fn builtin_assert(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("assert expects 1 argument, got {}", args.len()));
    }

    match &args[0] {
        Value::Bool(true) => Ok(Value::Unit),
        Value::Bool(false) => Err("assertion failed".to_string()),
        other => Err(format!(
            "assert expects bool, got {:?}",
            builtin_type_of(&[other.clone()])?
        )),
    }
}

fn builtin_assert_eq(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("assert_eq expects 2 arguments, got {}", args.len()));
    }

    if args[0] == args[1] {
        Ok(Value::Unit)
    } else {
        Err(format!(
            "assertion failed: `{:?}` != `{:?}`",
            args[0], args[1]
        ))
    }
}

fn builtin_panic(args: &[Value]) -> Result<Value, String> {
    let message = if args.is_empty() {
        "explicit panic".to_string()
    } else {
        args.iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(" ")
    };

    Err(format!("panic: {}", message))
}
```

---

## Module Exports

### src/environment/mod.rs

```rust
mod frame;
mod prelude;

pub use frame::ScopeGuard;

// Main Environment struct is in parent module (environment.rs)
```

### Update src/lib.rs

```rust
pub mod value;
pub mod environment;
pub mod error;

pub use value::{
    Value,
    StructValue,
    EnumValue,
    EnumData,
    FunctionValue,
    ClosureValue,
    BuiltinFn,
    CompiledFn,
    ValueRef,
    ValueRefMut,
    HashableValue,
};

pub use environment::{Environment, Binding, BindingMode, ScopeGuard};
pub use error::{TreebeardError, EnvironmentError};
```

---

## Test Cases

### tests/environment_tests.rs

```rust
use treebeard_core::*;

// ═══════════════════════════════════════════════════════════════════════
// Basic Operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_new_is_empty() {
    let env = Environment::new();
    assert!(env.is_empty());
    assert_eq!(env.len(), 0);
    assert_eq!(env.depth(), 1); // Global frame
}

#[test]
fn test_environment_define_and_get() {
    let mut env = Environment::new();
    env.define("x", Value::I64(42));

    assert_eq!(env.get("x"), Some(&Value::I64(42)));
    assert_eq!(env.get("y"), None);
    assert!(env.contains("x"));
    assert!(!env.contains("y"));
}

#[test]
fn test_environment_define_multiple() {
    let mut env = Environment::new();
    env.define("a", Value::I64(1));
    env.define("b", Value::I64(2));
    env.define("c", Value::I64(3));

    assert_eq!(env.len(), 3);
    assert_eq!(env.get("a"), Some(&Value::I64(1)));
    assert_eq!(env.get("b"), Some(&Value::I64(2)));
    assert_eq!(env.get("c"), Some(&Value::I64(3)));
}

// ═══════════════════════════════════════════════════════════════════════
// Scoping and Shadowing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_push_pop_frame() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("y", Value::I64(2));

    assert_eq!(env.depth(), 2);
    assert_eq!(env.get("x"), Some(&Value::I64(1)));
    assert_eq!(env.get("y"), Some(&Value::I64(2)));

    env.pop_frame();

    assert_eq!(env.depth(), 1);
    assert_eq!(env.get("x"), Some(&Value::I64(1)));
    assert_eq!(env.get("y"), None); // y is gone
}

#[test]
fn test_environment_shadowing() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("x", Value::I64(10)); // Shadow outer x

    assert_eq!(env.get("x"), Some(&Value::I64(10))); // Gets inner x

    env.pop_frame();

    assert_eq!(env.get("x"), Some(&Value::I64(1))); // Back to outer x
}

#[test]
fn test_environment_nested_scopes() {
    let mut env = Environment::new();
    env.define("a", Value::I64(1));

    env.push_frame(); // Depth 2
    env.define("b", Value::I64(2));

    env.push_frame(); // Depth 3
    env.define("c", Value::I64(3));

    assert_eq!(env.depth(), 3);
    assert!(env.contains("a"));
    assert!(env.contains("b"));
    assert!(env.contains("c"));

    env.pop_frame(); // Back to depth 2
    assert_eq!(env.depth(), 2);
    assert!(env.contains("a"));
    assert!(env.contains("b"));
    assert!(!env.contains("c"));

    env.pop_frame(); // Back to depth 1
    assert_eq!(env.depth(), 1);
    assert!(env.contains("a"));
    assert!(!env.contains("b"));
}

#[test]
fn test_environment_cannot_pop_global_frame() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    assert_eq!(env.depth(), 1);
    env.pop_frame(); // Should do nothing
    assert_eq!(env.depth(), 1);
    assert!(env.contains("x")); // x still there
}

#[test]
fn test_environment_contains_in_current_scope() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("y", Value::I64(2));

    assert!(env.contains("x")); // In outer scope
    assert!(env.contains("y")); // In current scope
    assert!(!env.contains_in_current_scope("x")); // x is in outer scope
    assert!(env.contains_in_current_scope("y")); // y is in current scope
}

// ═══════════════════════════════════════════════════════════════════════
// Mutability
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_mutable_binding() {
    let mut env = Environment::new();
    env.define_with_mode("x", Value::I64(1), BindingMode::Mutable);

    assert_eq!(env.get("x"), Some(&Value::I64(1)));

    env.assign("x", Value::I64(42)).unwrap();
    assert_eq!(env.get("x"), Some(&Value::I64(42)));
}

#[test]
fn test_environment_immutable_binding_error() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1)); // Immutable by default

    let result = env.assign("x", Value::I64(42));
    assert!(result.is_err());

    match result {
        Err(EnvironmentError::ImmutableBinding { name, .. }) => {
            assert_eq!(name, "x");
        }
        _ => panic!("Expected ImmutableBinding error"),
    }
}

#[test]
fn test_environment_assign_undefined_error() {
    let mut env = Environment::new();

    let result = env.assign("x", Value::I64(42));
    assert!(result.is_err());

    match result {
        Err(EnvironmentError::UndefinedVariable { name }) => {
            assert_eq!(name, "x");
        }
        _ => panic!("Expected UndefinedVariable error"),
    }
}

#[test]
fn test_environment_get_mut() {
    let mut env = Environment::new();
    env.define_with_mode("x", Value::I64(1), BindingMode::Mutable);

    {
        let value = env.get_mut("x").unwrap().unwrap();
        *value = Value::I64(100);
    }

    assert_eq!(env.get("x"), Some(&Value::I64(100)));
}

// ═══════════════════════════════════════════════════════════════════════
// Call Depth Tracking
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_call_depth() {
    let mut env = Environment::with_max_call_depth(5);

    assert_eq!(env.call_depth(), 0);

    env.enter_call().unwrap();
    assert_eq!(env.call_depth(), 1);

    env.enter_call().unwrap();
    env.enter_call().unwrap();
    assert_eq!(env.call_depth(), 3);

    env.exit_call();
    assert_eq!(env.call_depth(), 2);
}

#[test]
fn test_environment_stack_overflow() {
    let mut env = Environment::with_max_call_depth(3);

    env.enter_call().unwrap(); // 1
    env.enter_call().unwrap(); // 2
    env.enter_call().unwrap(); // 3

    let result = env.enter_call(); // 4 - should fail
    assert!(result.is_err());

    match result {
        Err(EnvironmentError::StackOverflow { depth, max }) => {
            assert_eq!(depth, 3);
            assert_eq!(max, 3);
        }
        _ => panic!("Expected StackOverflow error"),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Scope Guard
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_scope_guard_basic() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    {
        let mut guard = env.scope_guard();
        guard.define("y", Value::I64(2));
        assert!(guard.contains("y"));
    } // guard dropped, frame popped

    assert!(!env.contains("y"));
    assert!(env.contains("x"));
}

#[test]
fn test_scope_guard_nested() {
    let mut env = Environment::new();
    env.define("a", Value::I64(1));

    {
        let mut guard1 = env.scope_guard();
        guard1.define("b", Value::I64(2));

        {
            let mut guard2 = guard1.scope_guard();
            guard2.define("c", Value::I64(3));
            assert!(guard2.contains("c"));
        }

        assert!(!guard1.contains("c"));
        assert!(guard1.contains("b"));
    }

    assert!(!env.contains("b"));
    assert!(env.contains("a"));
}

// ═══════════════════════════════════════════════════════════════════════
// Iteration and Inspection
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_all_names() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));
    env.define("y", Value::I64(2));

    let names = env.all_names();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert_eq!(names.len(), 2);
}

#[test]
fn test_environment_names_in_current_scope() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("y", Value::I64(2));
    env.define("z", Value::I64(3));

    let current_names = env.names_in_current_scope();
    assert!(!current_names.contains(&"x"));
    assert!(current_names.contains(&"y"));
    assert!(current_names.contains(&"z"));
}

#[test]
fn test_environment_clear() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));
    env.push_frame();
    env.define("y", Value::I64(2));

    env.clear();

    assert!(env.is_empty());
    assert_eq!(env.depth(), 1);
    assert!(!env.contains("x"));
    assert!(!env.contains("y"));
}

// ═══════════════════════════════════════════════════════════════════════
// Prelude / Built-ins
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_with_prelude() {
    let env = Environment::with_prelude();

    assert!(env.contains("print"));
    assert!(env.contains("println"));
    assert!(env.contains("type_of"));
    assert!(env.contains("dbg"));
    assert!(env.contains("assert"));
    assert!(env.contains("assert_eq"));
    assert!(env.contains("panic"));
}

#[test]
fn test_builtin_type_of() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("type_of") {
        let result = (f.func)(&[Value::I64(42)]).unwrap();
        assert_eq!(result, Value::string("i64"));

        let result = (f.func)(&[Value::Bool(true)]).unwrap();
        assert_eq!(result, Value::string("bool"));
    } else {
        panic!("type_of not found");
    }
}

#[test]
fn test_builtin_assert_eq_pass() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert_eq") {
        let result = (f.func)(&[Value::I64(42), Value::I64(42)]);
        assert!(result.is_ok());
    } else {
        panic!("assert_eq not found");
    }
}

#[test]
fn test_builtin_assert_eq_fail() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert_eq") {
        let result = (f.func)(&[Value::I64(42), Value::I64(43)]);
        assert!(result.is_err());
    } else {
        panic!("assert_eq not found");
    }
}
```

---

## Completion Checklist

- [ ] Create `src/environment.rs` with `Environment` struct
- [ ] Implement `Binding` struct with name, value, mutable flag, span
- [ ] Implement `BindingMode` enum
- [ ] Implement frame management (`push_frame`, `pop_frame`)
- [ ] Implement binding definition (`define`, `define_with_mode`, `define_with_span`)
- [ ] Implement lookup (`get`, `get_binding`, `get_mut`, `contains`)
- [ ] Implement assignment with mutability checking
- [ ] Implement call depth tracking (`enter_call`, `exit_call`)
- [ ] Implement `ScopeGuard` for RAII scope management
- [ ] Implement `EnvironmentError` variants
- [ ] Implement prelude with basic built-in functions
- [ ] Add iteration methods (`iter`, `all_names`, `names_in_current_scope`)
- [ ] Update `lib.rs` exports
- [ ] All tests passing

---

## Design Notes

### Why Flat Scope Instead of Nested HashMaps?

1. **Cache-friendly**: Linear array access is faster than pointer chasing
2. **Simple cleanup**: `truncate()` is O(1) vs. dropping nested structures
3. **Predictable shadowing**: Reverse iteration naturally finds most recent binding
4. **Proven pattern**: Rhai uses this exact approach successfully

### Why Separate Call Depth from Frame Depth?

- Frame depth = lexical scope nesting (blocks, let expressions)
- Call depth = function call nesting (recursion detection)

A function body might push multiple frames (for inner blocks) but only increment call depth once.

### Why Store Span in Binding?

Error messages like "cannot assign to immutable binding `x`" are much more useful when they can point to where `x` was defined, not just where the error occurred.

---

## Next Stage

**Stage 1.3: Basic Expressions** — Implement evaluation for literals, paths (variable references), binary operations, and unary operations.
