//! Additional tests to improve coverage for Value type

use std::collections::HashMap;
use treebeard::*;

// ============================================================================
// Display/Debug Coverage - Testing all variants
// ============================================================================

#[test]
fn test_display_all_integer_types() {
    assert_eq!(format!("{:?}", Value::I8(42)), "42i8");
    assert_eq!(format!("{:?}", Value::I16(42)), "42i16");
    assert_eq!(format!("{:?}", Value::I32(42)), "42i32");
    assert_eq!(format!("{:?}", Value::I64(42)), "42");
    assert_eq!(format!("{:?}", Value::I128(42)), "42i128");
    assert_eq!(format!("{:?}", Value::Isize(42)), "42isize");

    assert_eq!(format!("{:?}", Value::U8(42)), "42u8");
    assert_eq!(format!("{:?}", Value::U16(42)), "42u16");
    assert_eq!(format!("{:?}", Value::U32(42)), "42u32");
    assert_eq!(format!("{:?}", Value::U64(42)), "42u64");
    assert_eq!(format!("{:?}", Value::U128(42)), "42u128");
    assert_eq!(format!("{:?}", Value::Usize(42)), "42usize");
}

#[test]
fn test_display_float_types() {
    assert_eq!(format!("{:?}", Value::F32(3.14)), "3.14f32");
    assert_eq!(format!("{:?}", Value::F64(3.14)), "3.14");
}

#[test]
fn test_display_char() {
    assert_eq!(format!("{:?}", Value::Char('x')), "'x'");
    assert_eq!(format!("{}", Value::Char('y')), "y"); // Display without quotes
}

#[test]
fn test_display_bytes() {
    let b = Value::bytes(vec![72u8, 101, 108, 108, 111]); // "Hello" in bytes
    let display = format!("{:?}", b);
    assert!(display.starts_with("b"));
}

#[test]
fn test_display_array() {
    let arr = Value::array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
    assert_eq!(format!("{:?}", arr), "[1, 2, 3]");
}

#[test]
fn test_display_hashmap() {
    let mut map = HashMap::new();
    map.insert(HashableValue(Value::string("key")), Value::I64(42));

    let hm = Value::HashMap(std::sync::Arc::new(map));
    let display = format!("{:?}", hm);
    assert!(display.contains("key"));
    assert!(display.contains("42"));
}

#[test]
fn test_display_empty_collections() {
    assert_eq!(format!("{:?}", Value::vec(vec![])), "vec![]");
    assert_eq!(format!("{:?}", Value::tuple(vec![])), "()");
    assert_eq!(format!("{:?}", Value::array(vec![])), "[]");
}

// ============================================================================
// From Implementation Coverage
// ============================================================================

#[test]
fn test_from_all_signed_integers() {
    assert_eq!(Value::from(42i8), Value::I8(42));
    assert_eq!(Value::from(42i16), Value::I16(42));
    assert_eq!(Value::from(42i32), Value::I32(42));
    assert_eq!(Value::from(42i64), Value::I64(42));
    assert_eq!(Value::from(42i128), Value::I128(42));
    assert_eq!(Value::from(42isize), Value::Isize(42));
}

#[test]
fn test_from_all_unsigned_integers() {
    assert_eq!(Value::from(42u8), Value::U8(42));
    assert_eq!(Value::from(42u16), Value::U16(42));
    assert_eq!(Value::from(42u32), Value::U32(42));
    assert_eq!(Value::from(42u64), Value::U64(42));
    assert_eq!(Value::from(42u128), Value::U128(42));
    assert_eq!(Value::from(42usize), Value::Usize(42));
}

#[test]
fn test_from_floats() {
    assert_eq!(Value::from(3.14f32), Value::F32(3.14));
    assert_eq!(Value::from(3.14f64), Value::F64(3.14));
}

#[test]
fn test_from_unit() {
    assert_eq!(Value::from(()), Value::Unit);
}

#[test]
fn test_from_char() {
    assert_eq!(Value::from('x'), Value::Char('x'));
}

#[test]
fn test_from_string_types() {
    let owned = String::from("hello");
    assert_eq!(Value::from(owned), Value::string("hello"));
    assert_eq!(Value::from("world"), Value::string("world"));
}

#[test]
fn test_from_vec_with_conversion() {
    let v: Value = vec![1i32, 2i32, 3i32].into();
    if let Some(items) = v.as_vec() {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], Value::I32(1));
    } else {
        panic!("Expected vec");
    }
}

#[test]
fn test_from_option_some() {
    let opt: Value = Some(42i64).into();
    assert!(matches!(opt, Value::Option(_)));
}

#[test]
fn test_from_option_none() {
    let opt: Value = Option::<i64>::None.into();
    assert!(matches!(opt, Value::Option(_)));
}

#[test]
fn test_from_result_ok() {
    let res: Value = Ok::<i64, String>(42).into();
    assert!(matches!(res, Value::Result(_)));
}

#[test]
fn test_from_result_err() {
    let res: Value = Err::<i64, String>("error".to_string()).into();
    assert!(matches!(res, Value::Result(_)));
}

// ============================================================================
// Extractor Coverage - Testing edge cases
// ============================================================================

#[test]
fn test_as_i64_unsigned_edge_cases() {
    // Small unsigned values convert to i64
    assert_eq!(Value::U8(255).as_i64(), Some(255));
    assert_eq!(Value::U16(65535).as_i64(), Some(65535));
    assert_eq!(Value::U32(100).as_i64(), Some(100));

    // Float doesn't convert
    assert_eq!(Value::F64(3.14).as_i64(), None);
    assert_eq!(Value::Bool(true).as_i64(), None);
}

#[test]
fn test_as_f64_edge_cases() {
    assert_eq!(Value::F32(3.14).as_f64(), Some(3.14f32 as f64));
    assert_eq!(Value::F64(2.71).as_f64(), Some(2.71));

    // Non-float doesn't convert
    assert_eq!(Value::I64(42).as_f64(), None);
    assert_eq!(Value::Bool(false).as_f64(), None);
}

#[test]
fn test_as_str_edge_cases() {
    assert_eq!(Value::string("test").as_str(), Some("test"));
    assert_eq!(Value::string("").as_str(), Some("")); // Empty string

    // Non-string doesn't convert
    assert_eq!(Value::I64(42).as_str(), None);
}

#[test]
fn test_as_bool_edge_cases() {
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Bool(false).as_bool(), Some(false));

    // Non-bool doesn't convert
    assert_eq!(Value::I64(1).as_bool(), None);
}

#[test]
fn test_as_vec_both_types() {
    let v = Value::vec(vec![Value::I64(1)]);
    assert_eq!(v.as_vec().map(|s| s.len()), Some(1));

    let a = Value::array(vec![Value::I64(1)]);
    assert_eq!(a.as_vec().map(|s| s.len()), Some(1));

    // Non-collection doesn't convert
    assert_eq!(Value::I64(42).as_vec(), None);
}

// ============================================================================
// Hashable Coverage
// ============================================================================

#[test]
fn test_hashable_all_integer_types() {
    assert!(HashableValue::is_hashable(&Value::I8(1)));
    assert!(HashableValue::is_hashable(&Value::I16(1)));
    assert!(HashableValue::is_hashable(&Value::I32(1)));
    assert!(HashableValue::is_hashable(&Value::I64(1)));
    assert!(HashableValue::is_hashable(&Value::I128(1)));
    assert!(HashableValue::is_hashable(&Value::Isize(1)));
    assert!(HashableValue::is_hashable(&Value::U8(1)));
    assert!(HashableValue::is_hashable(&Value::U16(1)));
    assert!(HashableValue::is_hashable(&Value::U32(1)));
    assert!(HashableValue::is_hashable(&Value::U64(1)));
    assert!(HashableValue::is_hashable(&Value::U128(1)));
    assert!(HashableValue::is_hashable(&Value::Usize(1)));
}

#[test]
fn test_hashable_primitives() {
    assert!(HashableValue::is_hashable(&Value::Unit));
    assert!(HashableValue::is_hashable(&Value::Bool(true)));
    assert!(HashableValue::is_hashable(&Value::Char('x')));
    assert!(HashableValue::is_hashable(&Value::string("test")));
    assert!(HashableValue::is_hashable(&Value::bytes(vec![1, 2, 3])));
}

#[test]
fn test_hashable_non_hashable_types() {
    assert!(!HashableValue::is_hashable(&Value::F32(3.14)));
    assert!(!HashableValue::is_hashable(&Value::F64(3.14)));
    assert!(!HashableValue::is_hashable(&Value::vec(vec![])));
    assert!(!HashableValue::is_hashable(&Value::tuple(vec![])));
    assert!(!HashableValue::is_hashable(&Value::array(vec![])));
}

#[test]
fn test_hashable_value_in_hashmap_all_types() {
    let mut map: HashMap<HashableValue, Value> = HashMap::new();

    // Test all hashable integer types
    map.insert(HashableValue(Value::I8(1)), Value::string("i8"));
    map.insert(HashableValue(Value::I16(2)), Value::string("i16"));
    map.insert(HashableValue(Value::I32(3)), Value::string("i32"));
    map.insert(HashableValue(Value::I64(4)), Value::string("i64"));
    map.insert(HashableValue(Value::U8(5)), Value::string("u8"));

    assert_eq!(map.len(), 5);
    assert_eq!(
        map.get(&HashableValue(Value::I8(1))),
        Some(&Value::string("i8"))
    );
}

#[test]
fn test_hashable_value_equality() {
    let h1 = HashableValue(Value::I64(42));
    let h2 = HashableValue(Value::I64(42));
    let h3 = HashableValue(Value::I64(43));

    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

#[test]
fn test_hashable_unit() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let h = HashableValue(Value::Unit);
    let mut hasher = DefaultHasher::new();
    h.hash(&mut hasher);
    let hash1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    h.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    assert_eq!(hash1, hash2);
}

#[test]
fn test_hashable_char() {
    let mut map: HashMap<HashableValue, Value> = HashMap::new();
    map.insert(HashableValue(Value::Char('a')), Value::I64(1));
    map.insert(HashableValue(Value::Char('b')), Value::I64(2));

    assert_eq!(
        map.get(&HashableValue(Value::Char('a'))),
        Some(&Value::I64(1))
    );
}

#[test]
fn test_hashable_bytes() {
    let mut map: HashMap<HashableValue, Value> = HashMap::new();
    map.insert(HashableValue(Value::bytes(vec![1, 2, 3])), Value::I64(42));

    assert_eq!(
        map.get(&HashableValue(Value::bytes(vec![1, 2, 3]))),
        Some(&Value::I64(42))
    );
}

// ============================================================================
// PartialEq Coverage - Testing all variants
// ============================================================================

#[test]
fn test_partialeq_all_integers() {
    // I types
    assert_eq!(Value::I8(42), Value::I8(42));
    assert_eq!(Value::I16(42), Value::I16(42));
    assert_eq!(Value::I32(42), Value::I32(42));
    assert_eq!(Value::I64(42), Value::I64(42));
    assert_eq!(Value::I128(42), Value::I128(42));
    assert_eq!(Value::Isize(42), Value::Isize(42));

    // U types
    assert_eq!(Value::U8(42), Value::U8(42));
    assert_eq!(Value::U16(42), Value::U16(42));
    assert_eq!(Value::U32(42), Value::U32(42));
    assert_eq!(Value::U64(42), Value::U64(42));
    assert_eq!(Value::U128(42), Value::U128(42));
    assert_eq!(Value::Usize(42), Value::Usize(42));
}

#[test]
fn test_partialeq_floats() {
    assert_eq!(Value::F32(3.14), Value::F32(3.14));
    assert_eq!(Value::F64(2.71), Value::F64(2.71));
}

#[test]
fn test_partialeq_char() {
    assert_eq!(Value::Char('a'), Value::Char('a'));
    assert_ne!(Value::Char('a'), Value::Char('b'));
}

#[test]
fn test_partialeq_bytes() {
    assert_eq!(Value::bytes(vec![1, 2, 3]), Value::bytes(vec![1, 2, 3]));
    assert_ne!(Value::bytes(vec![1, 2, 3]), Value::bytes(vec![1, 2]));
}

#[test]
fn test_partialeq_hashmap() {
    use std::sync::Arc;

    let mut map1 = HashMap::new();
    map1.insert(HashableValue(Value::I64(1)), Value::string("one"));

    let mut map2 = HashMap::new();
    map2.insert(HashableValue(Value::I64(1)), Value::string("one"));

    let hm1 = Value::HashMap(Arc::new(map1));
    let hm2 = Value::HashMap(Arc::new(map2));

    assert_eq!(hm1, hm2);
}

#[test]
fn test_partialeq_cross_type_inequality() {
    // Different types should never be equal
    assert_ne!(Value::I64(42), Value::U64(42));
    assert_ne!(Value::I64(42), Value::F64(42.0));
    assert_ne!(Value::Bool(true), Value::I64(1));
    assert_ne!(Value::Char('1'), Value::string("1"));
}

// ============================================================================
// Constructor Coverage
// ============================================================================

#[test]
fn test_constructor_string() {
    let s = Value::string("hello");
    assert!(matches!(s, Value::String(_)));
    assert_eq!(s.as_str(), Some("hello"));
}

#[test]
fn test_constructor_bytes() {
    let b = Value::bytes(vec![1u8, 2, 3]);
    assert!(matches!(b, Value::Bytes(_)));
}

#[test]
fn test_constructor_vec() {
    let v = Value::vec(vec![Value::I64(1), Value::I64(2)]);
    assert!(matches!(v, Value::Vec(_)));
}

#[test]
fn test_constructor_tuple() {
    let t = Value::tuple(vec![Value::I64(1), Value::string("a")]);
    assert!(matches!(t, Value::Tuple(_)));
}

#[test]
fn test_constructor_array() {
    let a = Value::array(vec![Value::I64(1), Value::I64(2)]);
    assert!(matches!(a, Value::Array(_)));
}

#[test]
fn test_constructor_struct() {
    let s = StructValue::new("Test");
    let v = Value::structure(s);
    assert!(matches!(v, Value::Struct(_)));
}

#[test]
fn test_constructor_enum() {
    let e = EnumValue::unit("Option", "None");
    let v = Value::enumeration(e);
    assert!(matches!(v, Value::Enum(_)));
}

#[test]
fn test_constructor_option_some() {
    let s = Value::some(Value::I64(42));
    assert!(matches!(s, Value::Option(_)));
}

#[test]
fn test_constructor_option_none() {
    let n = Value::none();
    assert!(matches!(n, Value::Option(_)));
}

#[test]
fn test_constructor_result_ok() {
    let ok = Value::ok(Value::I64(42));
    assert!(matches!(ok, Value::Result(_)));
}

#[test]
fn test_constructor_result_err() {
    let err = Value::err(Value::string("error"));
    assert!(matches!(err, Value::Result(_)));
}

// ============================================================================
// Type Predicates Coverage
// ============================================================================

#[test]
fn test_is_callable_false_cases() {
    assert!(!Value::I64(42).is_callable());
    assert!(!Value::string("test").is_callable());
    assert!(!Value::vec(vec![]).is_callable());
}
