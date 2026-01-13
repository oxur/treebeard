//! Compound value types: structs and enums

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
#[derive(Debug, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_value_new() {
        let s = StructValue::new("Point");

        assert_eq!(s.type_name, "Point");
        assert_eq!(s.fields.len(), 0);
        assert!(!s.is_tuple_struct);
    }

    #[test]
    fn test_struct_value_tuple() {
        let s = StructValue::tuple("Pair");

        assert_eq!(s.type_name, "Pair");
        assert_eq!(s.fields.len(), 0);
        assert!(s.is_tuple_struct);
    }

    #[test]
    fn test_struct_value_with_field() {
        let s = StructValue::new("Point")
            .with_field("x", Value::I64(10))
            .with_field("y", Value::I64(20));

        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.get("x"), Some(&Value::I64(10)));
        assert_eq!(s.get("y"), Some(&Value::I64(20)));
    }

    #[test]
    fn test_struct_value_get() {
        let mut s = StructValue::new("Person");
        s.fields.insert("name".to_string(), Value::string("Alice"));
        s.fields.insert("age".to_string(), Value::I64(30));

        assert_eq!(s.get("name"), Some(&Value::string("Alice")));
        assert_eq!(s.get("age"), Some(&Value::I64(30)));
        assert_eq!(s.get("missing"), None);
    }

    #[test]
    fn test_struct_value_get_index() {
        let s = StructValue::tuple("Pair")
            .with_field("0", Value::I64(1))
            .with_field("1", Value::I64(2));

        assert_eq!(s.get_index(0), Some(&Value::I64(1)));
        assert_eq!(s.get_index(1), Some(&Value::I64(2)));
        assert_eq!(s.get_index(2), None);
    }

    #[test]
    fn test_enum_value_unit() {
        let e = EnumValue::unit("Option", "None");

        assert_eq!(e.type_name, "Option");
        assert_eq!(e.variant, "None");
        assert_eq!(e.data, EnumData::Unit);
    }

    #[test]
    fn test_enum_value_tuple() {
        let e = EnumValue::tuple("Option", "Some", vec![Value::I64(42)]);

        assert_eq!(e.type_name, "Option");
        assert_eq!(e.variant, "Some");
        assert_eq!(e.data, EnumData::Tuple(vec![Value::I64(42)]));
    }

    #[test]
    fn test_enum_value_with_struct() {
        let mut fields = IndexMap::new();
        fields.insert("x".to_string(), Value::I64(10));
        fields.insert("y".to_string(), Value::I64(20));

        let e = EnumValue::with_struct("Message", "Move", fields.clone());

        assert_eq!(e.type_name, "Message");
        assert_eq!(e.variant, "Move");
        assert_eq!(e.data, EnumData::Struct(fields));
    }

    #[test]
    fn test_enum_value_is_variant() {
        let e1 = EnumValue::unit("Option", "None");
        let e2 = EnumValue::tuple("Option", "Some", vec![Value::I64(42)]);

        assert!(e1.is_variant("None"));
        assert!(!e1.is_variant("Some"));

        assert!(e2.is_variant("Some"));
        assert!(!e2.is_variant("None"));
    }

    #[test]
    fn test_enum_data_unit() {
        let data = EnumData::Unit;
        assert_eq!(data, EnumData::Unit);
    }

    #[test]
    fn test_enum_data_tuple() {
        let data = EnumData::Tuple(vec![Value::I64(1), Value::I64(2)]);
        assert_eq!(data, EnumData::Tuple(vec![Value::I64(1), Value::I64(2)]));
    }

    #[test]
    fn test_enum_data_struct() {
        let mut fields = IndexMap::new();
        fields.insert("a".to_string(), Value::I64(1));

        let data = EnumData::Struct(fields.clone());
        assert_eq!(data, EnumData::Struct(fields));
    }
}
