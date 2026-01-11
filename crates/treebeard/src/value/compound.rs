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
