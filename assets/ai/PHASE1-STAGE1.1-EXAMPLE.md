# Stage 1.1: Value Representation

**Phase:** 1 - Core Evaluator  
**Stage:** 1.1  
**Prerequisites:** None (this is the foundation)  
**Estimated effort:** 3-4 days

---

## Objective

Implement the `Value` enum that represents all Rust runtime values in the Treebeard interpreter. This is the foundational data structure that everything else builds upon.

---

## Overview

The `Value` type must represent every possible runtime value that Rust code can produce. We use a three-tier approach:

1. **Inline primitives** — small values stored directly in the enum (no allocation)
2. **Heap-allocated types** — larger values wrapped in `Arc` for sharing
3. **Callable types** — functions, closures, and builtins

The target size for `Value` is ~24 bytes. This balances inline storage capacity against enum overhead.

---

## Crate Setup

Create a new crate called `treebeard-core`:

```
treebeard-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── value.rs        ← This stage
│   ├── value/
│   │   ├── mod.rs
│   │   ├── primitive.rs
│   │   ├── compound.rs
│   │   ├── callable.rs
│   │   └── display.rs
│   └── error.rs        ← Basic error types
```

### Cargo.toml

```toml
[package]
name = "treebeard-core"
version = "0.1.0"
edition = "2021"
description = "Core types and evaluation for the Treebeard interpreter"

[dependencies]
syn = { version = "2", features = ["full", "parsing", "printing", "extra-traits"] }
proc-macro2 = "1"
quote = "1"
thiserror = "1"
indexmap = "2"          # Preserves insertion order for struct fields

[dev-dependencies]
pretty_assertions = "1"
```

---

## Core Type Definition

### src/value.rs

```rust
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use indexmap::IndexMap;
use syn::Ident;

/// Runtime value representation for the Treebeard interpreter.
///
/// Values are organized into three tiers:
/// - Tier 1: Inline primitives (no allocation)
/// - Tier 2: Heap-allocated compound types (Arc-wrapped)
/// - Tier 3: Callable types (functions, closures, builtins)
#[derive(Clone)]
pub enum Value {
    // ═══════════════════════════════════════════════════════════════════
    // Tier 1: Inline Primitives
    // ═══════════════════════════════════════════════════════════════════
    
    /// The unit type `()`
    Unit,
    
    /// Boolean: `true` or `false`
    Bool(bool),
    
    /// Unicode scalar value
    Char(char),
    
    // Signed integers
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    
    // Unsigned integers
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    
    // Floating point
    F32(f32),
    F64(f64),

    // ═══════════════════════════════════════════════════════════════════
    // Tier 2: Heap-Allocated Compound Types
    // ═══════════════════════════════════════════════════════════════════
    
    /// Heap-allocated string
    String(Arc<String>),
    
    /// Byte string (Vec<u8>)
    Bytes(Arc<Vec<u8>>),
    
    /// Homogeneous array/vec
    Vec(Arc<Vec<Value>>),
    
    /// Tuple (heterogeneous, fixed-size)
    Tuple(Arc<Vec<Value>>),
    
    /// Array (fixed-size, but we represent as Vec internally)
    Array(Arc<Vec<Value>>),
    
    /// Struct instance
    Struct(Arc<StructValue>),
    
    /// Enum variant instance
    Enum(Arc<EnumValue>),
    
    /// HashMap
    HashMap(Arc<HashMap<HashableValue, Value>>),
    
    /// Option<T> - special-cased for ergonomics
    Option(Arc<Option<Value>>),
    
    /// Result<T, E> - special-cased for ergonomics  
    Result(Arc<Result<Value, Value>>),

    // ═══════════════════════════════════════════════════════════════════
    // Tier 3: Callable Types (defined but not fully implemented this stage)
    // ═══════════════════════════════════════════════════════════════════
    
    /// User-defined function (from syn::ItemFn)
    Function(Arc<FunctionValue>),
    
    /// Closure with captured environment
    Closure(Arc<ClosureValue>),
    
    /// Built-in native function
    BuiltinFn(BuiltinFn),
    
    /// Compiled native function (escape hatch)
    CompiledFn(CompiledFn),

    // ═══════════════════════════════════════════════════════════════════
    // References (for ownership tracking - Phase 5)
    // ═══════════════════════════════════════════════════════════════════
    
    /// Immutable reference
    Ref(ValueRef),
    
    /// Mutable reference
    RefMut(ValueRefMut),
}
```

---

## Compound Type Definitions

### src/value/compound.rs

```rust
use std::sync::Arc;
use indexmap::IndexMap;
use super::Value;

/// A struct instance with named fields.
///
/// Uses IndexMap to preserve field order (important for tuple structs
/// and for predictable iteration).
#[derive(Debug, Clone)]
pub struct StructValue {
    /// The struct's type name (e.g., "Point", "Person")
    pub type_name: String,
    
    /// The struct's fields in definition order
    /// For tuple structs, keys are "0", "1", "2", etc.
    pub fields: IndexMap<String, Value>,
    
    /// Whether this is a tuple struct (fields accessed by index)
    pub is_tuple_struct: bool,
}

impl StructValue {
    /// Create a new named struct
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            fields: IndexMap::new(),
            is_tuple_struct: false,
        }
    }
    
    /// Create a new tuple struct
    pub fn tuple(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            fields: IndexMap::new(),
            is_tuple_struct: true,
        }
    }
    
    /// Add a field (builder pattern)
    pub fn with_field(mut self, name: impl Into<String>, value: Value) -> Self {
        self.fields.insert(name.into(), value);
        self
    }
    
    /// Get a field by name
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }
    
    /// Get a field by index (for tuple structs)
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        self.fields.get_index(index).map(|(_, v)| v)
    }
}

/// An enum variant instance.
#[derive(Debug, Clone)]
pub struct EnumValue {
    /// The enum's type name (e.g., "Option", "Result")
    pub type_name: String,
    
    /// The variant name (e.g., "Some", "None", "Ok", "Err")
    pub variant: String,
    
    /// The variant's data (if any)
    pub data: EnumData,
}

/// The data payload of an enum variant.
#[derive(Debug, Clone)]
pub enum EnumData {
    /// Unit variant: `None`, `Quit`
    Unit,
    
    /// Tuple variant: `Some(42)`, `Ok(value)`
    Tuple(Vec<Value>),
    
    /// Struct variant: `Message { x: 1, y: 2 }`
    Struct(IndexMap<String, Value>),
}

impl EnumValue {
    /// Create a unit variant
    pub fn unit(type_name: impl Into<String>, variant: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            variant: variant.into(),
            data: EnumData::Unit,
        }
    }
    
    /// Create a tuple variant
    pub fn tuple(
        type_name: impl Into<String>,
        variant: impl Into<String>,
        values: Vec<Value>,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            variant: variant.into(),
            data: EnumData::Tuple(values),
        }
    }
    
    /// Create a struct variant
    pub fn with_struct(
        type_name: impl Into<String>,
        variant: impl Into<String>,
        fields: IndexMap<String, Value>,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            variant: variant.into(),
            data: EnumData::Struct(fields),
        }
    }
    
    /// Check if this is a specific variant
    pub fn is_variant(&self, variant: &str) -> bool {
        self.variant == variant
    }
}
```

---

## Callable Type Definitions

### src/value/callable.rs

These types are defined now but will be fully implemented in later stages (1.5 for functions, 5.1 for closures).

```rust
use std::sync::Arc;
use super::Value;

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
    pub fn new(name: String, params: Vec<String>, body: syn::Block) -> Self {
        Self {
            name,
            params,
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
    pub func: Arc<dyn Fn(&[Value]) -> Result<Value, String> + Send + Sync>,
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
    
    /// Function pointer (loaded at runtime)
    /// This is a placeholder - actual implementation requires unsafe
    pub _marker: std::marker::PhantomData<()>,
}

impl std::fmt::Debug for CompiledFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompiledFn({} @ {:?})", self.name, self.lib_path)
    }
}
```

---

## Reference Types (Placeholders)

### src/value/refs.rs

These are placeholders for Phase 5 (Ownership). Define them now so the Value enum compiles.

```rust
use super::Value;
use std::sync::Arc;

/// An immutable reference to a value.
/// Full implementation in Phase 5.
#[derive(Debug, Clone)]
pub struct ValueRef {
    /// The referenced value
    pub value: Arc<Value>,
    
    /// Ownership tag (for tracking)
    pub tag: u32,
}

/// A mutable reference to a value.
/// Full implementation in Phase 5.
#[derive(Debug, Clone)]
pub struct ValueRefMut {
    /// The referenced value (interior mutability via Arc)
    pub value: Arc<std::sync::RwLock<Value>>,
    
    /// Ownership tag (for tracking)
    pub tag: u32,
}
```

---

## Hashable Value Wrapper

For using Values as HashMap keys, we need a hashable wrapper (since f32/f64 don't implement Hash).

### src/value/hashable.rs

```rust
use super::Value;
use std::hash::{Hash, Hasher};

/// A wrapper for Value that implements Hash and Eq.
///
/// Only primitive types and strings can be used as keys.
/// Attempting to hash a non-hashable type returns an error.
#[derive(Debug, Clone)]
pub struct HashableValue(pub Value);

impl HashableValue {
    /// Check if a value can be hashed
    pub fn is_hashable(value: &Value) -> bool {
        matches!(
            value,
            Value::Unit
                | Value::Bool(_)
                | Value::Char(_)
                | Value::I8(_)
                | Value::I16(_)
                | Value::I32(_)
                | Value::I64(_)
                | Value::I128(_)
                | Value::Isize(_)
                | Value::U8(_)
                | Value::U16(_)
                | Value::U32(_)
                | Value::U64(_)
                | Value::U128(_)
                | Value::Usize(_)
                | Value::String(_)
                | Value::Bytes(_)
        )
    }
}

impl Hash for HashableValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the discriminant first
        std::mem::discriminant(&self.0).hash(state);
        
        match &self.0 {
            Value::Unit => {}
            Value::Bool(b) => b.hash(state),
            Value::Char(c) => c.hash(state),
            Value::I8(n) => n.hash(state),
            Value::I16(n) => n.hash(state),
            Value::I32(n) => n.hash(state),
            Value::I64(n) => n.hash(state),
            Value::I128(n) => n.hash(state),
            Value::Isize(n) => n.hash(state),
            Value::U8(n) => n.hash(state),
            Value::U16(n) => n.hash(state),
            Value::U32(n) => n.hash(state),
            Value::U64(n) => n.hash(state),
            Value::U128(n) => n.hash(state),
            Value::Usize(n) => n.hash(state),
            Value::String(s) => s.hash(state),
            Value::Bytes(b) => b.hash(state),
            // Floats and compound types panic - should check is_hashable first
            _ => panic!("Attempted to hash non-hashable Value: {:?}", self.0),
        }
    }
}

impl PartialEq for HashableValue {
    fn eq(&self, other: &Self) -> bool {
        // Delegate to Value's PartialEq
        self.0 == other.0
    }
}

impl Eq for HashableValue {}
```

---

## Value Equality and Comparison

### Add to src/value.rs

```rust
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // Unit
            (Value::Unit, Value::Unit) => true,
            
            // Primitives
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            
            // Signed integers
            (Value::I8(a), Value::I8(b)) => a == b,
            (Value::I16(a), Value::I16(b)) => a == b,
            (Value::I32(a), Value::I32(b)) => a == b,
            (Value::I64(a), Value::I64(b)) => a == b,
            (Value::I128(a), Value::I128(b)) => a == b,
            (Value::Isize(a), Value::Isize(b)) => a == b,
            
            // Unsigned integers
            (Value::U8(a), Value::U8(b)) => a == b,
            (Value::U16(a), Value::U16(b)) => a == b,
            (Value::U32(a), Value::U32(b)) => a == b,
            (Value::U64(a), Value::U64(b)) => a == b,
            (Value::U128(a), Value::U128(b)) => a == b,
            (Value::Usize(a), Value::Usize(b)) => a == b,
            
            // Floats (use total_cmp for consistency, but eq for PartialEq)
            (Value::F32(a), Value::F32(b)) => a == b,
            (Value::F64(a), Value::F64(b)) => a == b,
            
            // Strings and bytes
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            
            // Collections (element-wise comparison)
            (Value::Vec(a), Value::Vec(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            
            // Structs (by type name and fields)
            (Value::Struct(a), Value::Struct(b)) => {
                a.type_name == b.type_name && a.fields == b.fields
            }
            
            // Enums (by type, variant, and data)
            (Value::Enum(a), Value::Enum(b)) => {
                a.type_name == b.type_name
                    && a.variant == b.variant
                    && match (&a.data, &b.data) {
                        (EnumData::Unit, EnumData::Unit) => true,
                        (EnumData::Tuple(a), EnumData::Tuple(b)) => a == b,
                        (EnumData::Struct(a), EnumData::Struct(b)) => a == b,
                        _ => false,
                    }
            }
            
            // Option and Result
            (Value::Option(a), Value::Option(b)) => a == b,
            (Value::Result(a), Value::Result(b)) => a == b,
            
            // Functions are equal if they're the same Arc
            (Value::Function(a), Value::Function(b)) => Arc::ptr_eq(a, b),
            (Value::Closure(a), Value::Closure(b)) => Arc::ptr_eq(a, b),
            
            // Builtins are equal if same name (identity)
            (Value::BuiltinFn(a), Value::BuiltinFn(b)) => a.name == b.name,
            
            // CompiledFn - by name and path
            (Value::CompiledFn(a), Value::CompiledFn(b)) => {
                a.name == b.name && a.lib_path == b.lib_path
            }
            
            // References - compare underlying values
            (Value::Ref(a), Value::Ref(b)) => a.value == b.value,
            
            // Different types are never equal
            _ => false,
        }
    }
}
```

---

## Display Implementation

### src/value/display.rs

```rust
use std::fmt;
use super::*;

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "'{}'", c),
            
            Value::I8(n) => write!(f, "{}i8", n),
            Value::I16(n) => write!(f, "{}i16", n),
            Value::I32(n) => write!(f, "{}i32", n),
            Value::I64(n) => write!(f, "{}", n),  // Default integer type
            Value::I128(n) => write!(f, "{}i128", n),
            Value::Isize(n) => write!(f, "{}isize", n),
            
            Value::U8(n) => write!(f, "{}u8", n),
            Value::U16(n) => write!(f, "{}u16", n),
            Value::U32(n) => write!(f, "{}u32", n),
            Value::U64(n) => write!(f, "{}u64", n),
            Value::U128(n) => write!(f, "{}u128", n),
            Value::Usize(n) => write!(f, "{}usize", n),
            
            Value::F32(n) => write!(f, "{}f32", n),
            Value::F64(n) => write!(f, "{}", n),  // Default float type
            
            Value::String(s) => write!(f, "{:?}", s.as_ref()),
            Value::Bytes(b) => write!(f, "b{:?}", b.as_ref()),
            
            Value::Vec(v) => {
                write!(f, "vec![")?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }
            
            Value::Tuple(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                if items.len() == 1 {
                    write!(f, ",")?;  // Single-element tuple needs trailing comma
                }
                write!(f, ")")
            }
            
            Value::Array(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }
            
            Value::Struct(s) => {
                write!(f, "{}", s.type_name)?;
                if s.is_tuple_struct {
                    write!(f, "(")?;
                    for (i, (_, v)) in s.fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{:?}", v)?;
                    }
                    write!(f, ")")
                } else {
                    write!(f, " {{ ")?;
                    for (i, (k, v)) in s.fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {:?}", k, v)?;
                    }
                    write!(f, " }}")
                }
            }
            
            Value::Enum(e) => {
                write!(f, "{}::{}", e.type_name, e.variant)?;
                match &e.data {
                    EnumData::Unit => Ok(()),
                    EnumData::Tuple(items) => {
                        write!(f, "(")?;
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{:?}", item)?;
                        }
                        write!(f, ")")
                    }
                    EnumData::Struct(fields) => {
                        write!(f, " {{ ")?;
                        for (i, (k, v)) in fields.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}: {:?}", k, v)?;
                        }
                        write!(f, " }}")
                    }
                }
            }
            
            Value::HashMap(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}: {:?}", k.0, v)?;
                }
                write!(f, "}}")
            }
            
            Value::Option(opt) => match opt.as_ref() {
                Some(v) => write!(f, "Some({:?})", v),
                None => write!(f, "None"),
            },
            
            Value::Result(res) => match res.as_ref() {
                Ok(v) => write!(f, "Ok({:?})", v),
                Err(e) => write!(f, "Err({:?})", e),
            },
            
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::BuiltinFn(b) => write!(f, "<builtin {}>", b.name),
            Value::CompiledFn(c) => write!(f, "<compiled {}>", c.name),
            
            Value::Ref(r) => write!(f, "&{:?}", r.value),
            Value::RefMut(r) => write!(f, "&mut <locked>"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display is more user-friendly, Debug is more detailed
        match self {
            Value::String(s) => write!(f, "{}", s.as_ref()),  // No quotes for Display
            Value::Char(c) => write!(f, "{}", c),  // No quotes for Display
            _ => fmt::Debug::fmt(self, f),
        }
    }
}
```

---

## Convenience Constructors

### Add to src/value.rs

```rust
impl Value {
    // ═══════════════════════════════════════════════════════════════════
    // Constructors
    // ═══════════════════════════════════════════════════════════════════
    
    /// Create a string value
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(Arc::new(s.into()))
    }
    
    /// Create a byte string value
    pub fn bytes(b: impl Into<Vec<u8>>) -> Self {
        Value::Bytes(Arc::new(b.into()))
    }
    
    /// Create a vec value
    pub fn vec(items: Vec<Value>) -> Self {
        Value::Vec(Arc::new(items))
    }
    
    /// Create a tuple value
    pub fn tuple(items: Vec<Value>) -> Self {
        Value::Tuple(Arc::new(items))
    }
    
    /// Create an array value
    pub fn array(items: Vec<Value>) -> Self {
        Value::Array(Arc::new(items))
    }
    
    /// Create a struct value
    pub fn structure(s: StructValue) -> Self {
        Value::Struct(Arc::new(s))
    }
    
    /// Create an enum value
    pub fn enumeration(e: EnumValue) -> Self {
        Value::Enum(Arc::new(e))
    }
    
    /// Create Some(value)
    pub fn some(value: Value) -> Self {
        Value::Option(Arc::new(Some(value)))
    }
    
    /// Create None
    pub fn none() -> Self {
        Value::Option(Arc::new(None))
    }
    
    /// Create Ok(value)
    pub fn ok(value: Value) -> Self {
        Value::Result(Arc::new(Ok(value)))
    }
    
    /// Create Err(value)
    pub fn err(value: Value) -> Self {
        Value::Result(Arc::new(Err(value)))
    }
    
    // ═══════════════════════════════════════════════════════════════════
    // Type Predicates
    // ═══════════════════════════════════════════════════════════════════
    
    pub fn is_unit(&self) -> bool { matches!(self, Value::Unit) }
    pub fn is_bool(&self) -> bool { matches!(self, Value::Bool(_)) }
    pub fn is_integer(&self) -> bool {
        matches!(self, 
            Value::I8(_) | Value::I16(_) | Value::I32(_) | Value::I64(_) |
            Value::I128(_) | Value::Isize(_) |
            Value::U8(_) | Value::U16(_) | Value::U32(_) | Value::U64(_) |
            Value::U128(_) | Value::Usize(_)
        )
    }
    pub fn is_float(&self) -> bool { matches!(self, Value::F32(_) | Value::F64(_)) }
    pub fn is_numeric(&self) -> bool { self.is_integer() || self.is_float() }
    pub fn is_string(&self) -> bool { matches!(self, Value::String(_)) }
    pub fn is_callable(&self) -> bool {
        matches!(self, 
            Value::Function(_) | Value::Closure(_) | 
            Value::BuiltinFn(_) | Value::CompiledFn(_)
        )
    }
    
    // ═══════════════════════════════════════════════════════════════════
    // Extractors (return Option for safe access)
    // ═══════════════════════════════════════════════════════════════════
    
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I8(n) => Some(*n as i64),
            Value::I16(n) => Some(*n as i64),
            Value::I32(n) => Some(*n as i64),
            Value::I64(n) => Some(*n),
            Value::Isize(n) => Some(*n as i64),
            // Unsigned that fit
            Value::U8(n) => Some(*n as i64),
            Value::U16(n) => Some(*n as i64),
            Value::U32(n) => Some(*n as i64),
            _ => None,
        }
    }
    
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F32(n) => Some(*n as f64),
            Value::F64(n) => Some(*n),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
    
    pub fn as_vec(&self) -> Option<&[Value]> {
        match self {
            Value::Vec(v) => Some(v.as_slice()),
            Value::Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}
```

---

## From Implementations

### Add to src/value.rs

```rust
// Primitive conversions
impl From<()> for Value { fn from(_: ()) -> Self { Value::Unit } }
impl From<bool> for Value { fn from(b: bool) -> Self { Value::Bool(b) } }
impl From<char> for Value { fn from(c: char) -> Self { Value::Char(c) } }

impl From<i8> for Value { fn from(n: i8) -> Self { Value::I8(n) } }
impl From<i16> for Value { fn from(n: i16) -> Self { Value::I16(n) } }
impl From<i32> for Value { fn from(n: i32) -> Self { Value::I32(n) } }
impl From<i64> for Value { fn from(n: i64) -> Self { Value::I64(n) } }
impl From<i128> for Value { fn from(n: i128) -> Self { Value::I128(n) } }
impl From<isize> for Value { fn from(n: isize) -> Self { Value::Isize(n) } }

impl From<u8> for Value { fn from(n: u8) -> Self { Value::U8(n) } }
impl From<u16> for Value { fn from(n: u16) -> Self { Value::U16(n) } }
impl From<u32> for Value { fn from(n: u32) -> Self { Value::U32(n) } }
impl From<u64> for Value { fn from(n: u64) -> Self { Value::U64(n) } }
impl From<u128> for Value { fn from(n: u128) -> Self { Value::U128(n) } }
impl From<usize> for Value { fn from(n: usize) -> Self { Value::Usize(n) } }

impl From<f32> for Value { fn from(n: f32) -> Self { Value::F32(n) } }
impl From<f64> for Value { fn from(n: f64) -> Self { Value::F64(n) } }

impl From<String> for Value { fn from(s: String) -> Self { Value::string(s) } }
impl From<&str> for Value { fn from(s: &str) -> Self { Value::string(s) } }

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::vec(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(opt: Option<T>) -> Self {
        Value::Option(Arc::new(opt.map(Into::into)))
    }
}

impl<T: Into<Value>, E: Into<Value>> From<Result<T, E>> for Value {
    fn from(res: Result<T, E>) -> Self {
        Value::Result(Arc::new(res.map(Into::into).map_err(Into::into)))
    }
}
```

---

## Module Structure

### src/lib.rs

```rust
pub mod value;
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
pub use error::TreebeardError;
```

### src/value/mod.rs

```rust
mod primitive;
mod compound;
mod callable;
mod refs;
mod display;
mod hashable;

pub use compound::{StructValue, EnumValue, EnumData};
pub use callable::{FunctionValue, ClosureValue, BuiltinFn, CompiledFn};
pub use refs::{ValueRef, ValueRefMut};
pub use hashable::HashableValue;

// Re-export Value from parent
use std::sync::Arc;
use std::collections::HashMap;

// Include the main Value enum definition here or in a separate file
include!("../value.rs");  // Or just define it directly in mod.rs
```

---

## Basic Error Types

### src/error.rs

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TreebeardError {
    #[error("Type error: expected {expected}, got {got}")]
    TypeError { expected: String, got: String },
    
    #[error("Value error: {0}")]
    ValueError(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}
```

---

## Test Cases

### tests/value_tests.rs

```rust
use treebeard_core::*;

#[test]
fn test_primitive_values() {
    // Unit
    assert_eq!(Value::Unit, Value::Unit);
    
    // Bool
    assert_eq!(Value::Bool(true), Value::Bool(true));
    assert_ne!(Value::Bool(true), Value::Bool(false));
    
    // Integers
    assert_eq!(Value::I64(42), Value::I64(42));
    assert_ne!(Value::I64(42), Value::I64(43));
    
    // Different integer types are not equal
    assert_ne!(Value::I32(42), Value::I64(42));
    
    // Floats
    assert_eq!(Value::F64(3.14), Value::F64(3.14));
}

#[test]
fn test_string_values() {
    let s1 = Value::string("hello");
    let s2 = Value::string("hello");
    let s3 = Value::string("world");
    
    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
    
    assert_eq!(s1.as_str(), Some("hello"));
}

#[test]
fn test_vec_values() {
    let v1 = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
    let v2 = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
    let v3 = Value::vec(vec![Value::I64(1), Value::I64(2)]);
    
    assert_eq!(v1, v2);
    assert_ne!(v1, v3);
}

#[test]
fn test_tuple_values() {
    let t1 = Value::tuple(vec![Value::I64(1), Value::string("hello")]);
    let t2 = Value::tuple(vec![Value::I64(1), Value::string("hello")]);
    
    assert_eq!(t1, t2);
}

#[test]
fn test_struct_values() {
    let s1 = StructValue::new("Point")
        .with_field("x", Value::I64(10))
        .with_field("y", Value::I64(20));
    
    let s2 = StructValue::new("Point")
        .with_field("x", Value::I64(10))
        .with_field("y", Value::I64(20));
    
    assert_eq!(Value::structure(s1.clone()), Value::structure(s2));
    
    assert_eq!(s1.get("x"), Some(&Value::I64(10)));
    assert_eq!(s1.get("z"), None);
}

#[test]
fn test_enum_values() {
    let none = EnumValue::unit("Option", "None");
    let some = EnumValue::tuple("Option", "Some", vec![Value::I64(42)]);
    
    assert!(none.is_variant("None"));
    assert!(some.is_variant("Some"));
    assert!(!some.is_variant("None"));
}

#[test]
fn test_option_values() {
    let some = Value::some(Value::I64(42));
    let none = Value::none();
    
    assert_ne!(some, none);
    
    // Two Somes with same value are equal
    assert_eq!(Value::some(Value::I64(42)), Value::some(Value::I64(42)));
}

#[test]
fn test_from_conversions() {
    assert_eq!(Value::from(42i64), Value::I64(42));
    assert_eq!(Value::from(true), Value::Bool(true));
    assert_eq!(Value::from("hello"), Value::string("hello"));
    
    let v: Value = vec![1i64, 2, 3].into();
    assert_eq!(v.as_vec().map(|v| v.len()), Some(3));
}

#[test]
fn test_type_predicates() {
    assert!(Value::Unit.is_unit());
    assert!(Value::Bool(true).is_bool());
    assert!(Value::I64(42).is_integer());
    assert!(Value::F64(3.14).is_float());
    assert!(Value::F64(3.14).is_numeric());
    assert!(Value::string("hello").is_string());
}

#[test]
fn test_display() {
    assert_eq!(format!("{:?}", Value::Unit), "()");
    assert_eq!(format!("{:?}", Value::Bool(true)), "true");
    assert_eq!(format!("{:?}", Value::I64(42)), "42");
    assert_eq!(format!("{:?}", Value::string("hello")), "\"hello\"");
    
    let tuple = Value::tuple(vec![Value::I64(1), Value::I64(2)]);
    assert_eq!(format!("{:?}", tuple), "(1, 2)");
    
    // Single-element tuple has trailing comma
    let single = Value::tuple(vec![Value::I64(1)]);
    assert_eq!(format!("{:?}", single), "(1,)");
}

#[test]
fn test_hashable_value() {
    use std::collections::HashMap;
    
    let mut map: HashMap<HashableValue, Value> = HashMap::new();
    map.insert(HashableValue(Value::string("key")), Value::I64(42));
    
    assert_eq!(
        map.get(&HashableValue(Value::string("key"))),
        Some(&Value::I64(42))
    );
    
    // Integers are hashable
    assert!(HashableValue::is_hashable(&Value::I64(42)));
    
    // Floats are not hashable
    assert!(!HashableValue::is_hashable(&Value::F64(3.14)));
    
    // Vecs are not hashable
    assert!(!HashableValue::is_hashable(&Value::vec(vec![])));
}

#[test]
fn test_value_size() {
    // Verify our size assumptions
    let size = std::mem::size_of::<Value>();
    println!("Value size: {} bytes", size);
    
    // Should be <= 32 bytes (24 target + some slack)
    assert!(size <= 32, "Value is too large: {} bytes", size);
}
```

---

## Completion Checklist

- [ ] Create `treebeard-core` crate with Cargo.toml
- [ ] Implement `Value` enum with all variants
- [ ] Implement `StructValue` with named and tuple struct support
- [ ] Implement `EnumValue` with unit, tuple, and struct variants
- [ ] Implement placeholder callable types (`FunctionValue`, `ClosureValue`, `BuiltinFn`, `CompiledFn`)
- [ ] Implement placeholder reference types (`ValueRef`, `ValueRefMut`)
- [ ] Implement `HashableValue` wrapper
- [ ] Implement `PartialEq` for `Value`
- [ ] Implement `Debug` and `Display` for `Value`
- [ ] Implement convenience constructors
- [ ] Implement `From` traits for common conversions
- [ ] Implement type predicates and extractors
- [ ] Basic `TreebeardError` type
- [ ] All tests passing
- [ ] Verify `Value` size is acceptable (~24-32 bytes)

---

## Next Stage

**Stage 1.2: Environment** — Implement scoped variable bindings with `Environment` struct, frame-based scoping, and variable lookup.
