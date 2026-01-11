//! Hashable wrapper for Value to enable use as HashMap keys

use std::hash::{Hash, Hasher};

use super::Value;

/// A wrapper for Value that implements Hash and Eq.
///
/// Only primitive types and strings can be used as keys.
/// Attempting to hash a non-hashable type will panic.
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
