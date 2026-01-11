//! Final coverage tests to reach 95%+ - targeting specific uncovered lines

use std::collections::HashMap;
use treebeard::*;

// ============================================================================
// Comprehensive Display Tests for Complex Types
// ============================================================================

#[test]
fn test_display_tuple_struct() {
    let s = StructValue::tuple("Color")
        .with_field("0", Value::U8(255))
        .with_field("1", Value::U8(128))
        .with_field("2", Value::U8(64));

    let display = format!("{:?}", Value::structure(s));
    assert!(display.contains("Color"));
    assert!(display.contains("255"));
    assert!(display.contains("128"));
    assert!(display.contains("64"));
}

#[test]
fn test_display_named_struct() {
    let s = StructValue::new("Person")
        .with_field("name", Value::string("Alice"))
        .with_field("age", Value::U32(30));

    let display = format!("{:?}", Value::structure(s));
    assert!(display.contains("Person"));
    assert!(display.contains("name"));
    assert!(display.contains("Alice"));
    assert!(display.contains("age"));
    assert!(display.contains("30"));
}

#[test]
fn test_display_enum_unit_variant() {
    let e = EnumValue::unit("Color", "Red");
    let display = format!("{:?}", Value::enumeration(e));
    assert_eq!(display, "Color::Red");
}

#[test]
fn test_display_enum_tuple_variant() {
    let e = EnumValue::tuple("Result", "Ok", vec![Value::I64(42)]);
    let display = format!("{:?}", Value::enumeration(e));
    assert_eq!(display, "Result::Ok(42)");
}

#[test]
fn test_display_enum_struct_variant() {
    use indexmap::IndexMap;
    let mut fields = IndexMap::new();
    fields.insert("x".to_string(), Value::I64(10));
    fields.insert("y".to_string(), Value::I64(20));

    let e = EnumValue::with_struct("Message", "Move", fields);
    let display = format!("{:?}", Value::enumeration(e));
    assert!(display.contains("Message::Move"));
    assert!(display.contains("x: 10"));
    assert!(display.contains("y: 20"));
}

#[test]
fn test_display_nested_tuples() {
    let inner = Value::tuple(vec![Value::I64(1), Value::I64(2)]);
    let outer = Value::tuple(vec![inner, Value::I64(3)]);
    let display = format!("{:?}", outer);
    assert!(display.contains("(1, 2)"));
}

#[test]
fn test_display_nested_vecs() {
    let inner = Value::vec(vec![Value::I64(1), Value::I64(2)]);
    let outer = Value::vec(vec![inner, Value::I64(3)]);
    let display = format!("{:?}", outer);
    assert!(display.contains("vec!"));
}

#[test]
fn test_display_mixed_nested() {
    let tuple = Value::tuple(vec![Value::I64(1), Value::string("test")]);
    let vec = Value::vec(vec![tuple, Value::Bool(true)]);
    let display = format!("{:?}", vec);
    assert!(display.contains("test"));
    assert!(display.contains("true"));
}

// ============================================================================
// Additional PartialEq Tests for Full Coverage
// ============================================================================

#[test]
fn test_equality_nested_structs() {
    let inner1 = StructValue::new("Inner").with_field("val", Value::I64(42));
    let outer1 = StructValue::new("Outer").with_field("inner", Value::structure(inner1));

    let inner2 = StructValue::new("Inner").with_field("val", Value::I64(42));
    let outer2 = StructValue::new("Outer").with_field("inner", Value::structure(inner2));

    assert_eq!(Value::structure(outer1), Value::structure(outer2));
}

#[test]
fn test_inequality_different_struct_names() {
    let s1 = StructValue::new("TypeA").with_field("x", Value::I64(1));
    let s2 = StructValue::new("TypeB").with_field("x", Value::I64(1));

    assert_ne!(Value::structure(s1), Value::structure(s2));
}

#[test]
fn test_inequality_different_enum_types() {
    let e1 = EnumValue::unit("TypeA", "Variant");
    let e2 = EnumValue::unit("TypeB", "Variant");

    assert_ne!(Value::enumeration(e1), Value::enumeration(e2));
}

#[test]
fn test_inequality_different_enum_variants() {
    let e1 = EnumValue::unit("Type", "VariantA");
    let e2 = EnumValue::unit("Type", "VariantB");

    assert_ne!(Value::enumeration(e1), Value::enumeration(e2));
}

#[test]
fn test_inequality_enum_data_mismatch() {
    let e1 = EnumValue::tuple("Type", "Variant", vec![Value::I64(1)]);
    let e2 = EnumValue::unit("Type", "Variant");

    assert_ne!(Value::enumeration(e1), Value::enumeration(e2));
}

#[test]
fn test_equality_enum_tuple_data() {
    let e1 = EnumValue::tuple("Type", "Var", vec![Value::I64(1), Value::I64(2)]);
    let e2 = EnumValue::tuple("Type", "Var", vec![Value::I64(1), Value::I64(2)]);

    assert_eq!(Value::enumeration(e1), Value::enumeration(e2));
}

#[test]
fn test_inequality_enum_tuple_data() {
    let e1 = EnumValue::tuple("Type", "Var", vec![Value::I64(1)]);
    let e2 = EnumValue::tuple("Type", "Var", vec![Value::I64(2)]);

    assert_ne!(Value::enumeration(e1), Value::enumeration(e2));
}

#[test]
fn test_equality_enum_struct_data() {
    use indexmap::IndexMap;
    let mut fields1 = IndexMap::new();
    fields1.insert("a".to_string(), Value::I64(1));

    let mut fields2 = IndexMap::new();
    fields2.insert("a".to_string(), Value::I64(1));

    let e1 = EnumValue::with_struct("Type", "Var", fields1);
    let e2 = EnumValue::with_struct("Type", "Var", fields2);

    assert_eq!(Value::enumeration(e1), Value::enumeration(e2));
}

#[test]
fn test_inequality_enum_struct_data() {
    use indexmap::IndexMap;
    let mut fields1 = IndexMap::new();
    fields1.insert("a".to_string(), Value::I64(1));

    let mut fields2 = IndexMap::new();
    fields2.insert("a".to_string(), Value::I64(2));

    let e1 = EnumValue::with_struct("Type", "Var", fields1);
    let e2 = EnumValue::with_struct("Type", "Var", fields2);

    assert_ne!(Value::enumeration(e1), Value::enumeration(e2));
}

// ============================================================================
// Additional From Tests
// ============================================================================

#[test]
fn test_from_empty_vec() {
    let v: Value = Vec::<i64>::new().into();
    assert_eq!(v.as_vec().map(|s| s.len()), Some(0));
}

#[test]
fn test_from_nested_vec() {
    let inner: Value = vec![1i64, 2].into();
    let outer: Value = vec![inner].into();
    assert_eq!(outer.as_vec().map(|s| s.len()), Some(1));
}

// ============================================================================
// Additional Hashable Tests
// ============================================================================

#[test]
fn test_hash_bool() {
    let mut map: HashMap<HashableValue, Value> = HashMap::new();
    map.insert(HashableValue(Value::Bool(true)), Value::I64(1));
    map.insert(HashableValue(Value::Bool(false)), Value::I64(0));

    assert_eq!(map.len(), 2);
    assert_eq!(
        map.get(&HashableValue(Value::Bool(true))),
        Some(&Value::I64(1))
    );
}

#[test]
fn test_hash_all_integer_types_in_map() {
    let mut map: HashMap<HashableValue, &str> = HashMap::new();

    map.insert(HashableValue(Value::I8(1)), "i8");
    map.insert(HashableValue(Value::I16(1)), "i16");
    map.insert(HashableValue(Value::I32(1)), "i32");
    map.insert(HashableValue(Value::I64(1)), "i64");
    map.insert(HashableValue(Value::I128(1)), "i128");
    map.insert(HashableValue(Value::Isize(1)), "isize");
    map.insert(HashableValue(Value::U8(1)), "u8");
    map.insert(HashableValue(Value::U16(1)), "u16");
    map.insert(HashableValue(Value::U32(1)), "u32");
    map.insert(HashableValue(Value::U64(1)), "u64");
    map.insert(HashableValue(Value::U128(1)), "u128");
    map.insert(HashableValue(Value::Usize(1)), "usize");

    // All different integer types with value 1 should be distinct keys
    assert_eq!(map.len(), 12);
}

// ============================================================================
// Edge Cases for Extractors
// ============================================================================

#[test]
fn test_as_i64_isize() {
    assert_eq!(Value::Isize(100).as_i64(), Some(100));
}

#[test]
fn test_as_i64_i128_in_range() {
    // I128 doesn't convert to i64 in our implementation
    assert_eq!(Value::I128(42).as_i64(), None);
}

#[test]
fn test_as_i64_u64_large() {
    // Large U64 doesn't convert
    assert_eq!(Value::U64(u64::MAX).as_i64(), None);
}

#[test]
fn test_extractors_on_wrong_types() {
    let s = Value::string("test");

    assert_eq!(s.as_bool(), None);
    assert_eq!(s.as_i64(), None);
    assert_eq!(s.as_f64(), None);
    assert_eq!(s.as_vec(), None);
}

// ============================================================================
// Additional Type Predicate Tests
// ============================================================================

#[test]
fn test_is_unit_on_non_unit() {
    assert!(!Value::I64(42).is_unit());
    assert!(!Value::Bool(false).is_unit());
}

#[test]
fn test_is_bool_on_non_bool() {
    assert!(!Value::I64(1).is_bool());
    assert!(!Value::Unit.is_bool());
}

#[test]
fn test_is_string_on_non_string() {
    assert!(!Value::I64(42).is_string());
    assert!(!Value::Char('a').is_string());
}

#[test]
fn test_is_numeric_comprehensive() {
    // All integer types
    assert!(Value::I8(1).is_numeric());
    assert!(Value::I16(1).is_numeric());
    assert!(Value::I32(1).is_numeric());
    assert!(Value::I64(1).is_numeric());
    assert!(Value::I128(1).is_numeric());
    assert!(Value::Isize(1).is_numeric());
    assert!(Value::U8(1).is_numeric());
    assert!(Value::U16(1).is_numeric());
    assert!(Value::U32(1).is_numeric());
    assert!(Value::U64(1).is_numeric());
    assert!(Value::U128(1).is_numeric());
    assert!(Value::Usize(1).is_numeric());

    // Float types
    assert!(Value::F32(1.0).is_numeric());
    assert!(Value::F64(1.0).is_numeric());

    // Non-numeric
    assert!(!Value::Bool(true).is_numeric());
    assert!(!Value::string("123").is_numeric());
}
