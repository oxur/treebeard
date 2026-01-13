//! Display and Debug implementations for Value

use std::fmt;

use super::*;

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "'{}'", c),

            Value::I8(n) => write!(f, "{}i8", n),
            Value::I16(n) => write!(f, "{}i16", n),
            Value::I32(n) => write!(f, "{}i32", n),
            Value::I64(n) => write!(f, "{}", n), // Default integer type
            Value::I128(n) => write!(f, "{}i128", n),
            Value::Isize(n) => write!(f, "{}isize", n),

            Value::U8(n) => write!(f, "{}u8", n),
            Value::U16(n) => write!(f, "{}u16", n),
            Value::U32(n) => write!(f, "{}u32", n),
            Value::U64(n) => write!(f, "{}u64", n),
            Value::U128(n) => write!(f, "{}u128", n),
            Value::Usize(n) => write!(f, "{}usize", n),

            Value::F32(n) => write!(f, "{}f32", n),
            Value::F64(n) => write!(f, "{}", n), // Default float type

            Value::String(s) => write!(f, "{:?}", s.as_ref()),
            Value::Bytes(b) => write!(f, "b{:?}", b.as_ref()),

            Value::Vec(v) => {
                write!(f, "vec![")?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }

            Value::Tuple(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                if items.len() == 1 {
                    write!(f, ",")?; // Single-element tuple needs trailing comma
                }
                write!(f, ")")
            }

            Value::Array(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }

            Value::Struct(s) => {
                write!(f, "{}", s.type_name)?;
                if s.is_tuple_struct {
                    write!(f, "(")?;
                    for (i, (_, v)) in s.fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{:?}", v)?;
                    }
                    write!(f, ")")
                } else {
                    write!(f, " {{ ")?;
                    for (i, (k, v)) in s.fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {:?}", k, v)?;
                    }
                    write!(f, " }}")
                }
            }

            Value::Enum(e) => {
                write!(f, "{}::{}", e.type_name, e.variant)?;
                match &e.data {
                    EnumData::Unit => Ok(()),
                    EnumData::Tuple(items) => {
                        write!(f, "(")?;
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{:?}", item)?;
                        }
                        write!(f, ")")
                    }
                    EnumData::Struct(fields) => {
                        write!(f, " {{ ")?;
                        for (i, (k, v)) in fields.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}: {:?}", k, v)?;
                        }
                        write!(f, " }}")
                    }
                }
            }

            Value::HashMap(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}: {:?}", k.0, v)?;
                }
                write!(f, "}}")
            }

            Value::Option(opt) => match opt.as_ref() {
                Some(v) => write!(f, "Some({:?})", v),
                None => write!(f, "None"),
            },

            Value::Result(res) => match res.as_ref() {
                Ok(v) => write!(f, "Ok({:?})", v),
                Err(e) => write!(f, "Err({:?})", e),
            },

            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::BuiltinFn(b) => write!(f, "<builtin {}>", b.name),
            Value::CompiledFn(c) => write!(f, "<compiled {}>", c.name),

            Value::Ref(r) => write!(f, "&{:?}", r.value),
            Value::RefMut(_) => write!(f, "&mut <locked>"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display is more user-friendly, Debug is more detailed
        match self {
            Value::String(s) => write!(f, "{}", s.as_ref()), // No quotes for Display
            Value::Char(c) => write!(f, "{}", c),            // No quotes for Display
            _ => fmt::Debug::fmt(self, f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_debug_unit() {
        assert_eq!(format!("{:?}", Value::Unit), "()");
    }

    #[test]
    fn test_debug_bool() {
        assert_eq!(format!("{:?}", Value::Bool(true)), "true");
        assert_eq!(format!("{:?}", Value::Bool(false)), "false");
    }

    #[test]
    fn test_debug_char() {
        assert_eq!(format!("{:?}", Value::Char('a')), "'a'");
    }

    #[test]
    fn test_debug_integers() {
        assert_eq!(format!("{:?}", Value::I8(42)), "42i8");
        assert_eq!(format!("{:?}", Value::I16(42)), "42i16");
        assert_eq!(format!("{:?}", Value::I32(42)), "42i32");
        assert_eq!(format!("{:?}", Value::I64(42)), "42");
        assert_eq!(format!("{:?}", Value::U8(42)), "42u8");
        assert_eq!(format!("{:?}", Value::U32(42)), "42u32");
    }

    #[test]
    fn test_debug_floats() {
        assert_eq!(format!("{:?}", Value::F32(1.5)), "1.5f32");
        assert_eq!(format!("{:?}", Value::F64(2.5)), "2.5");
    }

    #[test]
    fn test_debug_string() {
        assert_eq!(format!("{:?}", Value::string("hello")), "\"hello\"");
    }

    #[test]
    fn test_debug_vec() {
        let v = Value::Vec(Arc::new(vec![Value::I64(1), Value::I64(2)]));
        assert_eq!(format!("{:?}", v), "vec![1, 2]");
    }

    #[test]
    fn test_debug_tuple() {
        let t = Value::Tuple(Arc::new(vec![Value::I64(1), Value::string("hi")]));
        let formatted = format!("{:?}", t);
        assert!(formatted.contains("1"));
        assert!(formatted.contains("hi"));
    }

    #[test]
    fn test_debug_option_some() {
        let opt = Value::Option(Arc::new(Some(Value::I64(42))));
        let formatted = format!("{:?}", opt);
        assert!(formatted.contains("Some"));
        assert!(formatted.contains("42"));
    }

    #[test]
    fn test_debug_option_none() {
        let opt = Value::Option(Arc::new(None));
        assert_eq!(format!("{:?}", opt), "None");
    }

    #[test]
    fn test_debug_result_ok() {
        let res = Value::Result(Arc::new(Ok(Value::I64(42))));
        let formatted = format!("{:?}", res);
        assert!(formatted.contains("Ok"));
        assert!(formatted.contains("42"));
    }

    #[test]
    fn test_debug_result_err() {
        let res = Value::Result(Arc::new(Err(Value::string("error"))));
        let formatted = format!("{:?}", res);
        assert!(formatted.contains("Err"));
        assert!(formatted.contains("error"));
    }

    #[test]
    fn test_display_string() {
        // Display doesn't include quotes
        assert_eq!(format!("{}", Value::string("hello")), "hello");
    }

    #[test]
    fn test_display_char() {
        // Display doesn't include quotes
        assert_eq!(format!("{}", Value::Char('a')), "a");
    }

    #[test]
    fn test_display_integer() {
        // Display falls back to Debug for non-string/char types
        assert_eq!(format!("{}", Value::I64(42)), "42");
    }
}
