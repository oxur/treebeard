//! Comprehensive tests for Value type

use treebeard::*;

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
fn test_tuple_struct_values() {
    let s1 = StructValue::tuple("Pair")
        .with_field("0", Value::I64(1))
        .with_field("1", Value::I64(2));

    assert!(s1.is_tuple_struct);
    assert_eq!(s1.get_index(0), Some(&Value::I64(1)));
    assert_eq!(s1.get_index(1), Some(&Value::I64(2)));
    assert_eq!(s1.get_index(2), None);
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
fn test_enum_struct_variant() {
    use indexmap::IndexMap;
    let mut fields = IndexMap::new();
    fields.insert("x".to_string(), Value::I64(10));
    fields.insert("y".to_string(), Value::I64(20));

    let variant = EnumValue::with_struct("Message", "Point", fields);
    assert!(variant.is_variant("Point"));

    if let EnumData::Struct(ref fields) = variant.data {
        assert_eq!(fields.get("x"), Some(&Value::I64(10)));
    } else {
        panic!("Expected struct variant");
    }
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
fn test_result_values() {
    let ok = Value::ok(Value::I64(42));
    let err = Value::err(Value::string("error"));

    assert_ne!(ok, err);

    // Two Oks with same value are equal
    assert_eq!(Value::ok(Value::I64(42)), Value::ok(Value::I64(42)));
}

#[test]
fn test_from_conversions() {
    assert_eq!(Value::from(42i64), Value::I64(42));
    assert_eq!(Value::from(true), Value::Bool(true));
    assert_eq!(Value::from("hello"), Value::string("hello"));

    let v: Value = vec![1i64, 2, 3].into();
    assert_eq!(v.as_vec().map(|v| v.len()), Some(3));

    // Option conversion
    let opt: Value = Some(42i64).into();
    assert!(matches!(opt, Value::Option(_)));

    // Result conversion
    let res: Value = Ok::<i64, String>(42).into();
    assert!(matches!(res, Value::Result(_)));
}

#[test]
fn test_type_predicates() {
    assert!(Value::Unit.is_unit());
    assert!(Value::Bool(true).is_bool());
    assert!(Value::I64(42).is_integer());
    assert!(Value::I32(42).is_integer());
    assert!(Value::F64(3.14).is_float());
    assert!(Value::F64(3.14).is_numeric());
    assert!(Value::I64(42).is_numeric());
    assert!(Value::string("hello").is_string());
}

#[test]
fn test_extractors() {
    // Bool
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::I64(42).as_bool(), None);

    // i64
    assert_eq!(Value::I64(42).as_i64(), Some(42));
    assert_eq!(Value::I32(42).as_i64(), Some(42));
    assert_eq!(Value::U8(42).as_i64(), Some(42));
    assert_eq!(Value::Bool(true).as_i64(), None);

    // f64
    assert_eq!(Value::F64(3.14).as_f64(), Some(3.14));
    assert_eq!(Value::F32(3.14).as_f64(), Some(3.14f32 as f64));
    assert_eq!(Value::I64(42).as_f64(), None);

    // str
    assert_eq!(Value::string("hello").as_str(), Some("hello"));
    assert_eq!(Value::I64(42).as_str(), None);

    // vec
    let v = Value::vec(vec![Value::I64(1), Value::I64(2)]);
    assert_eq!(v.as_vec().map(|s| s.len()), Some(2));
}

#[test]
fn test_display_unit() {
    assert_eq!(format!("{:?}", Value::Unit), "()");
}

#[test]
fn test_display_primitives() {
    assert_eq!(format!("{:?}", Value::Bool(true)), "true");
    assert_eq!(format!("{:?}", Value::I64(42)), "42");
    assert_eq!(format!("{:?}", Value::I32(42)), "42i32");
    assert_eq!(format!("{:?}", Value::F64(3.14)), "3.14");
    assert_eq!(format!("{:?}", Value::string("hello")), "\"hello\"");
}

#[test]
fn test_display_tuple() {
    let tuple = Value::tuple(vec![Value::I64(1), Value::I64(2)]);
    assert_eq!(format!("{:?}", tuple), "(1, 2)");

    // Single-element tuple has trailing comma
    let single = Value::tuple(vec![Value::I64(1)]);
    assert_eq!(format!("{:?}", single), "(1,)");
}

#[test]
fn test_display_vec() {
    let v = Value::vec(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
    assert_eq!(format!("{:?}", v), "vec![1, 2, 3]");
}

#[test]
fn test_display_struct() {
    let s = StructValue::new("Point")
        .with_field("x", Value::I64(10))
        .with_field("y", Value::I64(20));

    let display = format!("{:?}", Value::structure(s));
    assert!(display.contains("Point"));
    assert!(display.contains("x: 10"));
    assert!(display.contains("y: 20"));
}

#[test]
fn test_display_enum() {
    let e = EnumValue::tuple("Option", "Some", vec![Value::I64(42)]);
    let display = format!("{:?}", Value::enumeration(e));
    assert_eq!(display, "Option::Some(42)");

    let none = EnumValue::unit("Option", "None");
    assert_eq!(format!("{:?}", Value::enumeration(none)), "Option::None");
}

#[test]
fn test_display_option() {
    assert_eq!(format!("{:?}", Value::some(Value::I64(42))), "Some(42)");
    assert_eq!(format!("{:?}", Value::none()), "None");
}

#[test]
fn test_display_result() {
    assert_eq!(format!("{:?}", Value::ok(Value::I64(42))), "Ok(42)");
    assert_eq!(
        format!("{:?}", Value::err(Value::string("error"))),
        "Err(\"error\")"
    );
}

#[test]
fn test_display_vs_debug_string() {
    let s = Value::string("hello");
    // Display shows string without quotes
    assert_eq!(format!("{}", s), "hello");
    // Debug shows string with quotes
    assert_eq!(format!("{:?}", s), "\"hello\"");
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
#[should_panic(expected = "Attempted to hash non-hashable Value")]
fn test_hashable_value_panic_on_float() {
    use std::collections::HashMap;

    let mut map: HashMap<HashableValue, Value> = HashMap::new();
    // This should panic because floats can't be hashed
    map.insert(HashableValue(Value::F64(3.14)), Value::I64(42));
}

#[test]
fn test_value_size() {
    // Verify our size assumptions
    let size = std::mem::size_of::<Value>();
    println!("Value size: {} bytes", size);

    // The enum has many variants (20+), and largest variant (I128/U128) is 16 bytes
    // Actual size is 64 bytes on 64-bit systems, which is acceptable for an interpreter
    // (8 bytes discriminant + 56 bytes data, rounded up for alignment)
    // This is a reasonable trade-off for supporting all Rust integer types directly
    assert!(
        size <= 64,
        "Value is unexpectedly large: {} bytes (expected ~64)",
        size
    );
}

#[test]
fn test_char_values() {
    let c1 = Value::Char('a');
    let c2 = Value::Char('a');
    let c3 = Value::Char('b');

    assert_eq!(c1, c2);
    assert_ne!(c1, c3);
    assert_eq!(format!("{:?}", c1), "'a'");
}

#[test]
fn test_bytes_values() {
    let b1 = Value::bytes(vec![1u8, 2, 3]);
    let b2 = Value::bytes(vec![1u8, 2, 3]);
    let b3 = Value::bytes(vec![1u8, 2]);

    assert_eq!(b1, b2);
    assert_ne!(b1, b3);
}

#[test]
fn test_array_values() {
    let a1 = Value::array(vec![Value::I64(1), Value::I64(2)]);
    let a2 = Value::array(vec![Value::I64(1), Value::I64(2)]);

    assert_eq!(a1, a2);
    assert_eq!(a1.as_vec().map(|s| s.len()), Some(2));
}

#[test]
fn test_all_integer_types() {
    assert!(Value::I8(1).is_integer());
    assert!(Value::I16(1).is_integer());
    assert!(Value::I32(1).is_integer());
    assert!(Value::I64(1).is_integer());
    assert!(Value::I128(1).is_integer());
    assert!(Value::Isize(1).is_integer());
    assert!(Value::U8(1).is_integer());
    assert!(Value::U16(1).is_integer());
    assert!(Value::U32(1).is_integer());
    assert!(Value::U64(1).is_integer());
    assert!(Value::U128(1).is_integer());
    assert!(Value::Usize(1).is_integer());
}

#[test]
fn test_integer_equality() {
    // Same type, same value
    assert_eq!(Value::I32(42), Value::I32(42));

    // Different types are not equal even if values are same
    assert_ne!(Value::I32(42), Value::I64(42));
    assert_ne!(Value::U32(42), Value::I32(42));
}

#[test]
fn test_float_types() {
    assert!(Value::F32(1.0).is_float());
    assert!(Value::F64(1.0).is_float());
    assert!(Value::F32(1.0).is_numeric());
    assert!(Value::F64(1.0).is_numeric());
}

#[test]
fn test_nested_values() {
    let nested = Value::vec(vec![
        Value::tuple(vec![Value::I64(1), Value::string("a")]),
        Value::tuple(vec![Value::I64(2), Value::string("b")]),
    ]);

    if let Some(items) = nested.as_vec() {
        assert_eq!(items.len(), 2);
    } else {
        panic!("Expected vec");
    }
}

#[test]
fn test_clone_value() {
    let v1 = Value::string("hello");
    let v2 = v1.clone();

    // Cloning creates equal values
    assert_eq!(v1, v2);

    // Arc means they share the same heap allocation
    if let (Value::String(s1), Value::String(s2)) = (&v1, &v2) {
        assert!(std::sync::Arc::ptr_eq(s1, s2));
    }
}
