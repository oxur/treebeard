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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_is_hashable_primitives() {
        assert!(HashableValue::is_hashable(&Value::Unit));
        assert!(HashableValue::is_hashable(&Value::Bool(true)));
        assert!(HashableValue::is_hashable(&Value::Char('x')));
        assert!(HashableValue::is_hashable(&Value::I8(42)));
        assert!(HashableValue::is_hashable(&Value::I16(42)));
        assert!(HashableValue::is_hashable(&Value::I32(42)));
        assert!(HashableValue::is_hashable(&Value::I64(42)));
        assert!(HashableValue::is_hashable(&Value::I128(42)));
        assert!(HashableValue::is_hashable(&Value::Isize(42)));
        assert!(HashableValue::is_hashable(&Value::U8(42)));
        assert!(HashableValue::is_hashable(&Value::U16(42)));
        assert!(HashableValue::is_hashable(&Value::U32(42)));
        assert!(HashableValue::is_hashable(&Value::U64(42)));
        assert!(HashableValue::is_hashable(&Value::U128(42)));
        assert!(HashableValue::is_hashable(&Value::Usize(42)));
    }

    #[test]
    fn test_is_hashable_strings() {
        assert!(HashableValue::is_hashable(&Value::string("hello")));
        assert!(HashableValue::is_hashable(&Value::bytes(vec![1, 2, 3])));
    }

    #[test]
    fn test_is_not_hashable_floats() {
        assert!(!HashableValue::is_hashable(&Value::F32(3.14)));
        assert!(!HashableValue::is_hashable(&Value::F64(3.14)));
    }

    #[test]
    fn test_is_not_hashable_compound() {
        use std::sync::Arc;
        assert!(!HashableValue::is_hashable(&Value::vec(vec![Value::I64(
            1
        )])));
        assert!(!HashableValue::is_hashable(&Value::tuple(vec![
            Value::I64(1)
        ])));
        assert!(!HashableValue::is_hashable(&Value::Option(Arc::new(Some(
            Value::I64(1)
        )))));
        assert!(!HashableValue::is_hashable(&Value::Option(Arc::new(None))));
    }

    #[test]
    fn test_hash_unit() {
        let v1 = HashableValue(Value::Unit);
        let v2 = HashableValue(Value::Unit);

        let mut set = HashSet::new();
        set.insert(v1);
        // v2 should be equal to v1
        assert!(!set.insert(v2));
    }

    #[test]
    fn test_hash_integers() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::I64(42)), "forty-two");
        map.insert(HashableValue(Value::I64(100)), "hundred");

        assert_eq!(map.get(&HashableValue(Value::I64(42))), Some(&"forty-two"));
        assert_eq!(map.get(&HashableValue(Value::I64(100))), Some(&"hundred"));
        assert_eq!(map.get(&HashableValue(Value::I64(99))), None);
    }

    #[test]
    fn test_hash_different_int_types() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::I8(42)), "i8");
        map.insert(HashableValue(Value::I16(42)), "i16");
        map.insert(HashableValue(Value::I32(42)), "i32");
        map.insert(HashableValue(Value::I64(42)), "i64");
        map.insert(HashableValue(Value::U8(42)), "u8");

        // Different types with same numeric value should be different keys
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn test_hash_strings() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::string("hello")), 1);
        map.insert(HashableValue(Value::string("world")), 2);

        assert_eq!(map.get(&HashableValue(Value::string("hello"))), Some(&1));
        assert_eq!(map.get(&HashableValue(Value::string("world"))), Some(&2));
        assert_eq!(map.get(&HashableValue(Value::string("foo"))), None);
    }

    #[test]
    fn test_hash_bools() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::Bool(true)), "true");
        map.insert(HashableValue(Value::Bool(false)), "false");

        assert_eq!(map.get(&HashableValue(Value::Bool(true))), Some(&"true"));
        assert_eq!(map.get(&HashableValue(Value::Bool(false))), Some(&"false"));
    }

    #[test]
    fn test_hash_chars() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::Char('a')), 1);
        map.insert(HashableValue(Value::Char('b')), 2);

        assert_eq!(map.get(&HashableValue(Value::Char('a'))), Some(&1));
        assert_eq!(map.get(&HashableValue(Value::Char('b'))), Some(&2));
    }

    #[test]
    fn test_hash_bytes() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::bytes(vec![1, 2, 3])), "123");
        map.insert(HashableValue(Value::bytes(vec![4, 5, 6])), "456");

        assert_eq!(
            map.get(&HashableValue(Value::bytes(vec![1, 2, 3]))),
            Some(&"123")
        );
        assert_eq!(
            map.get(&HashableValue(Value::bytes(vec![4, 5, 6]))),
            Some(&"456")
        );
    }

    #[test]
    #[should_panic(expected = "Attempted to hash non-hashable Value")]
    fn test_hash_float_panics() {
        let v = HashableValue(Value::F64(3.14));
        let mut map = HashMap::new();
        map.insert(v, "should panic");
    }

    #[test]
    #[should_panic(expected = "Attempted to hash non-hashable Value")]
    fn test_hash_vec_panics() {
        let v = HashableValue(Value::vec(vec![Value::I64(1)]));
        let mut map = HashMap::new();
        map.insert(v, "should panic");
    }

    #[test]
    fn test_eq_same_value() {
        let v1 = HashableValue(Value::I64(42));
        let v2 = HashableValue(Value::I64(42));
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_eq_different_values() {
        let v1 = HashableValue(Value::I64(42));
        let v2 = HashableValue(Value::I64(43));
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_eq_different_types() {
        let v1 = HashableValue(Value::I64(42));
        let v2 = HashableValue(Value::I32(42));
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_hash_set_deduplication() {
        let mut set = HashSet::new();
        set.insert(HashableValue(Value::I64(42)));
        set.insert(HashableValue(Value::I64(42))); // Duplicate
        set.insert(HashableValue(Value::I64(43)));

        assert_eq!(set.len(), 2); // Only 2 unique values
    }

    #[test]
    fn test_hash_all_unsigned_types() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::U8(1)), "u8");
        map.insert(HashableValue(Value::U16(2)), "u16");
        map.insert(HashableValue(Value::U32(3)), "u32");
        map.insert(HashableValue(Value::U64(4)), "u64");
        map.insert(HashableValue(Value::U128(5)), "u128");
        map.insert(HashableValue(Value::Usize(6)), "usize");

        assert_eq!(map.len(), 6);
        assert_eq!(map.get(&HashableValue(Value::U8(1))), Some(&"u8"));
        assert_eq!(map.get(&HashableValue(Value::U128(5))), Some(&"u128"));
    }

    #[test]
    fn test_hash_all_signed_types() {
        let mut map = HashMap::new();
        map.insert(HashableValue(Value::I8(-1)), "i8");
        map.insert(HashableValue(Value::I16(-2)), "i16");
        map.insert(HashableValue(Value::I32(-3)), "i32");
        map.insert(HashableValue(Value::I64(-4)), "i64");
        map.insert(HashableValue(Value::I128(-5)), "i128");
        map.insert(HashableValue(Value::Isize(-6)), "isize");

        assert_eq!(map.len(), 6);
        assert_eq!(map.get(&HashableValue(Value::I8(-1))), Some(&"i8"));
        assert_eq!(map.get(&HashableValue(Value::I128(-5))), Some(&"i128"));
    }
}
