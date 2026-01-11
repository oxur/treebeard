//! Reference types for ownership tracking (placeholders for Phase 5)

use std::sync::Arc;

use super::Value;

/// An immutable reference to a value.
///
/// Full implementation in Phase 5 (Ownership).
#[derive(Debug, Clone)]
pub struct ValueRef {
    /// The referenced value
    pub value: Arc<Value>,

    /// Ownership tag (for tracking)
    pub tag: u32,
}

/// A mutable reference to a value.
///
/// Full implementation in Phase 5 (Ownership).
#[derive(Debug, Clone)]
pub struct ValueRefMut {
    /// The referenced value (interior mutability via Arc)
    pub value: Arc<std::sync::RwLock<Value>>,

    /// Ownership tag (for tracking)
    pub tag: u32,
}
