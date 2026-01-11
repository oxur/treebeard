//! Value representation for runtime values

mod callable;
mod compound;
mod display;
mod hashable;
mod impls;
mod refs;

pub use callable::{BuiltinFn, BuiltinFnPtr, ClosureValue, CompiledFn, FunctionValue};
pub use compound::{EnumData, EnumValue, StructValue};
pub use hashable::HashableValue;
pub use refs::{ValueRef, ValueRefMut};

use std::collections::HashMap;
use std::sync::Arc;

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
    /// 8-bit signed integer
    I8(i8),
    /// 16-bit signed integer
    I16(i16),
    /// 32-bit signed integer
    I32(i32),
    /// 64-bit signed integer (default integer type)
    I64(i64),
    /// 128-bit signed integer
    I128(i128),
    /// Pointer-sized signed integer
    Isize(isize),

    // Unsigned integers
    /// 8-bit unsigned integer
    U8(u8),
    /// 16-bit unsigned integer
    U16(u16),
    /// 32-bit unsigned integer
    U32(u32),
    /// 64-bit unsigned integer
    U64(u64),
    /// 128-bit unsigned integer
    U128(u128),
    /// Pointer-sized unsigned integer
    Usize(usize),

    // Floating point
    /// 32-bit floating point
    F32(f32),
    /// 64-bit floating point (default float type)
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

// SAFETY: Value is safe to Send across threads because:
// - All primitive types are Send
// - All heap types are wrapped in Arc, which provides thread-safe reference counting
// - syn::Block and syn::Expr (in callable types) are Send
// - The only interior mutability is in ValueRefMut via RwLock, which is Send
unsafe impl Send for Value {}

// SAFETY: Value is safe to share references across threads because:
// - All primitive types are Sync
// - All heap types are wrapped in Arc, which is Sync
// - We never expose mutable references without proper synchronization (RwLock in ValueRefMut)
unsafe impl Sync for Value {}
