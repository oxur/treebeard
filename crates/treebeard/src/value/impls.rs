//! Value trait implementations: constructors, predicates, extractors, From traits, PartialEq

use std::sync::Arc;

use super::*;

// ═══════════════════════════════════════════════════════════════════
// Convenience Constructors
// ═══════════════════════════════════════════════════════════════════

impl Value {
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
    /// Check if value is unit type
    pub fn is_unit(&self) -> bool {
        matches!(self, Value::Unit)
    }

    /// Check if value is boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if value is any integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Value::I8(_)
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
        )
    }

    /// Check if value is any float type
    pub fn is_float(&self) -> bool {
        matches!(self, Value::F32(_) | Value::F64(_))
    }

    /// Check if value is numeric (integer or float)
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Check if value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if value is callable (function, closure, or builtin)
    pub fn is_callable(&self) -> bool {
        matches!(
            self,
            Value::Function(_) | Value::Closure(_) | Value::BuiltinFn(_) | Value::CompiledFn(_)
        )
    }

    // ═══════════════════════════════════════════════════════════════════
    // Extractors (return Option for safe access)
    // ═══════════════════════════════════════════════════════════════════
    /// Extract boolean value
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Extract as i64 (converts from smaller integer types)
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

    /// Extract as usize (converts from integer types)
    pub fn as_usize(&self) -> Option<usize> {
        match self {
            Value::Usize(n) => Some(*n),
            Value::U8(n) => Some(*n as usize),
            Value::U16(n) => Some(*n as usize),
            Value::U32(n) => Some(*n as usize),
            Value::U64(n) => (*n).try_into().ok(),
            Value::U128(n) => (*n).try_into().ok(),
            // Signed integers (if non-negative)
            Value::I8(n) if *n >= 0 => Some(*n as usize),
            Value::I16(n) if *n >= 0 => Some(*n as usize),
            Value::I32(n) if *n >= 0 => Some(*n as usize),
            Value::I64(n) if *n >= 0 => Some(*n as usize),
            Value::I128(n) if *n >= 0 => (*n).try_into().ok(),
            Value::Isize(n) if *n >= 0 => Some(*n as usize),
            _ => None,
        }
    }

    /// Extract as f64 (converts from f32)
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F32(n) => Some(*n as f64),
            Value::F64(n) => Some(*n),
            _ => None,
        }
    }

    /// Extract string slice
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Extract vec or array as slice
    pub fn as_vec(&self) -> Option<&[Value]> {
        match self {
            Value::Vec(v) => Some(v.as_slice()),
            Value::Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// PartialEq Implementation
// ═══════════════════════════════════════════════════════════════════

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

            // Floats (use bitwise equality for PartialEq)
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

            // HashMap
            (Value::HashMap(a), Value::HashMap(b)) => a == b,

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

// ═══════════════════════════════════════════════════════════════════
// From Trait Implementations
// ═══════════════════════════════════════════════════════════════════

// Primitive conversions
impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Unit
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<char> for Value {
    fn from(c: char) -> Self {
        Value::Char(c)
    }
}

impl From<i8> for Value {
    fn from(n: i8) -> Self {
        Value::I8(n)
    }
}

impl From<i16> for Value {
    fn from(n: i16) -> Self {
        Value::I16(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::I32(n)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::I64(n)
    }
}

impl From<i128> for Value {
    fn from(n: i128) -> Self {
        Value::I128(n)
    }
}

impl From<isize> for Value {
    fn from(n: isize) -> Self {
        Value::Isize(n)
    }
}

impl From<u8> for Value {
    fn from(n: u8) -> Self {
        Value::U8(n)
    }
}

impl From<u16> for Value {
    fn from(n: u16) -> Self {
        Value::U16(n)
    }
}

impl From<u32> for Value {
    fn from(n: u32) -> Self {
        Value::U32(n)
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Value::U64(n)
    }
}

impl From<u128> for Value {
    fn from(n: u128) -> Self {
        Value::U128(n)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Usize(n)
    }
}

impl From<f32> for Value {
    fn from(n: f32) -> Self {
        Value::F32(n)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::F64(n)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::string(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::string(s)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // Constructors
    #[test]
    fn test_string_constructor() {
        let v = Value::string("hello");
        assert!(matches!(v, Value::String(_)));
    }

    #[test]
    fn test_vec_constructor() {
        let v = Value::vec(vec![Value::I64(1), Value::I64(2)]);
        assert!(matches!(v, Value::Vec(_)));
    }

    #[test]
    fn test_tuple_constructor() {
        let v = Value::tuple(vec![Value::I64(1), Value::Bool(true)]);
        assert!(matches!(v, Value::Tuple(_)));
    }

    #[test]
    fn test_some_constructor() {
        let v = Value::some(Value::I64(42));
        assert!(matches!(v, Value::Option(_)));
    }

    #[test]
    fn test_none_constructor() {
        let v = Value::none();
        match v {
            Value::Option(opt) => assert!(opt.is_none()),
            _ => panic!("Expected Option"),
        }
    }

    #[test]
    fn test_ok_constructor() {
        let v = Value::ok(Value::I64(42));
        assert!(matches!(v, Value::Result(_)));
    }

    #[test]
    fn test_err_constructor() {
        let v = Value::err(Value::string("error"));
        assert!(matches!(v, Value::Result(_)));
    }

    // Predicates
    #[test]
    fn test_is_unit() {
        assert!(Value::Unit.is_unit());
        assert!(!Value::I64(42).is_unit());
    }

    #[test]
    fn test_is_bool() {
        assert!(Value::Bool(true).is_bool());
        assert!(!Value::I64(42).is_bool());
    }

    #[test]
    fn test_is_integer() {
        assert!(Value::I64(42).is_integer());
        assert!(Value::U32(10).is_integer());
        assert!(!Value::F64(1.5).is_integer());
    }

    #[test]
    fn test_is_float() {
        assert!(Value::F64(1.5).is_float());
        assert!(Value::F32(2.5).is_float());
        assert!(!Value::I64(42).is_float());
    }

    #[test]
    fn test_is_numeric() {
        assert!(Value::I64(42).is_numeric());
        assert!(Value::F64(1.5).is_numeric());
        assert!(!Value::string("hi").is_numeric());
    }

    #[test]
    fn test_is_string() {
        assert!(Value::string("hello").is_string());
        assert!(!Value::I64(42).is_string());
    }

    // Extractors
    #[test]
    fn test_as_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::I64(42).as_bool(), None);
    }

    #[test]
    fn test_as_i64() {
        assert_eq!(Value::I64(42).as_i64(), Some(42));
        assert_eq!(Value::I32(10).as_i64(), Some(10));
        assert_eq!(Value::string("hi").as_i64(), None);
    }

    #[test]
    fn test_as_usize() {
        assert_eq!(Value::Usize(42).as_usize(), Some(42));
        assert_eq!(Value::U64(10).as_usize(), Some(10));
        assert_eq!(Value::I64(-1).as_usize(), None); // Negative
    }

    #[test]
    fn test_as_f64() {
        assert_eq!(Value::F64(1.5).as_f64(), Some(1.5));
        assert_eq!(Value::F32(2.5).as_f64(), Some(2.5));
        assert_eq!(Value::string("hi").as_f64(), None);
    }

    #[test]
    fn test_as_str() {
        let v = Value::string("hello");
        assert_eq!(v.as_str(), Some("hello"));
        assert_eq!(Value::I64(42).as_str(), None);
    }

    #[test]
    fn test_as_vec() {
        let v = Value::vec(vec![Value::I64(1), Value::I64(2)]);
        let vec_ref = v.as_vec().unwrap();
        assert_eq!(vec_ref.len(), 2);
        assert_eq!(Value::I64(42).as_vec(), None);
    }

    // PartialEq
    #[test]
    fn test_partialeq_primitives() {
        assert_eq!(Value::Unit, Value::Unit);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_eq!(Value::I64(42), Value::I64(42));
        assert_ne!(Value::I64(42), Value::I64(43));
    }

    #[test]
    fn test_partialeq_collections() {
        let v1 = Value::vec(vec![Value::I64(1), Value::I64(2)]);
        let v2 = Value::vec(vec![Value::I64(1), Value::I64(2)]);
        let v3 = Value::vec(vec![Value::I64(1), Value::I64(3)]);
        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_partialeq_option() {
        let some1 = Value::some(Value::I64(42));
        let some2 = Value::some(Value::I64(42));
        let some3 = Value::some(Value::I64(43));
        let none1 = Value::none();
        let none2 = Value::none();

        assert_eq!(some1, some2);
        assert_ne!(some1, some3);
        assert_eq!(none1, none2);
        assert_ne!(some1, none1);
    }

    // From trait
    #[test]
    fn test_from_unit() {
        let v: Value = ().into();
        assert_eq!(v, Value::Unit);
    }

    #[test]
    fn test_from_bool() {
        let v: Value = true.into();
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn test_from_integers() {
        assert_eq!(Value::from(42i64), Value::I64(42));
        assert_eq!(Value::from(42i32), Value::I32(42));
        assert_eq!(Value::from(42u64), Value::U64(42));
    }

    #[test]
    fn test_from_floats() {
        assert_eq!(Value::from(1.5f64), Value::F64(1.5));
        assert_eq!(Value::from(2.5f32), Value::F32(2.5));
    }

    #[test]
    fn test_from_string() {
        let v: Value = "hello".into();
        assert_eq!(v, Value::string("hello"));
    }

    #[test]
    fn test_from_vec() {
        let v: Value = vec![1i64, 2i64, 3i64].into();
        match v {
            Value::Vec(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected Vec"),
        }
    }

    #[test]
    fn test_from_option() {
        let v: Value = Some(42i64).into();
        match v {
            Value::Option(opt) => assert!(opt.is_some()),
            _ => panic!("Expected Option"),
        }
    }

    #[test]
    fn test_from_result() {
        let v: Value = Ok::<i64, String>(42).into();
        match v {
            Value::Result(res) => assert!(res.is_ok()),
            _ => panic!("Expected Result"),
        }
    }
}
