//! Standard prelude with built-in functions

use super::Environment;
use crate::value::{BuiltinFn, Value};
use std::sync::Arc;

impl Environment {
    /// Create an environment with standard built-in functions.
    pub fn with_prelude() -> Self {
        let mut env = Self::new();
        env.load_prelude();
        env
    }

    /// Load the standard prelude into this environment.
    pub fn load_prelude(&mut self) {
        // Printing
        self.define_builtin(BuiltinFn {
            name: "print".to_string(),
            arity: -1, // Variadic
            func: Arc::new(builtin_print),
        });

        self.define_builtin(BuiltinFn {
            name: "println".to_string(),
            arity: -1,
            func: Arc::new(builtin_println),
        });

        // Type inspection
        self.define_builtin(BuiltinFn {
            name: "type_of".to_string(),
            arity: 1,
            func: Arc::new(builtin_type_of),
        });

        // Debug representation
        self.define_builtin(BuiltinFn {
            name: "dbg".to_string(),
            arity: 1,
            func: Arc::new(builtin_dbg),
        });

        // Assertions
        self.define_builtin(BuiltinFn {
            name: "assert".to_string(),
            arity: 1,
            func: Arc::new(builtin_assert),
        });

        self.define_builtin(BuiltinFn {
            name: "assert_eq".to_string(),
            arity: 2,
            func: Arc::new(builtin_assert_eq),
        });

        // Panic
        self.define_builtin(BuiltinFn {
            name: "panic".to_string(),
            arity: -1,
            func: Arc::new(builtin_panic),
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Built-in Function Implementations
// ═══════════════════════════════════════════════════════════════════════

fn builtin_print(args: &[Value]) -> Result<Value, String> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg);
    }
    Ok(Value::Unit)
}

fn builtin_println(args: &[Value]) -> Result<Value, String> {
    builtin_print(args)?;
    println!();
    Ok(Value::Unit)
}

fn builtin_type_of(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("type_of expects 1 argument, got {}", args.len()));
    }

    let type_name = match &args[0] {
        Value::Unit => "()",
        Value::Bool(_) => "bool",
        Value::Char(_) => "char",
        Value::I8(_) => "i8",
        Value::I16(_) => "i16",
        Value::I32(_) => "i32",
        Value::I64(_) => "i64",
        Value::I128(_) => "i128",
        Value::Isize(_) => "isize",
        Value::U8(_) => "u8",
        Value::U16(_) => "u16",
        Value::U32(_) => "u32",
        Value::U64(_) => "u64",
        Value::U128(_) => "u128",
        Value::Usize(_) => "usize",
        Value::F32(_) => "f32",
        Value::F64(_) => "f64",
        Value::String(_) => "String",
        Value::Bytes(_) => "Vec<u8>",
        Value::Vec(_) => "Vec",
        Value::Tuple(_) => "tuple",
        Value::Array(_) => "array",
        Value::Struct(s) => return Ok(Value::string(&s.type_name)),
        Value::Enum(e) => return Ok(Value::string(&e.type_name)),
        Value::HashMap(_) => "HashMap",
        Value::Option(_) => "Option",
        Value::Result(_) => "Result",
        Value::Function(_) => "fn",
        Value::Closure(_) => "closure",
        Value::BuiltinFn(_) => "builtin_fn",
        Value::CompiledFn(_) => "compiled_fn",
        Value::Ref(_) => "ref",
        Value::RefMut(_) => "ref_mut",
    };

    Ok(Value::string(type_name))
}

fn builtin_dbg(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("dbg expects 1 argument, got {}", args.len()));
    }

    eprintln!("[dbg] {:?}", args[0]);
    Ok(args[0].clone())
}

fn builtin_assert(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("assert expects 1 argument, got {}", args.len()));
    }

    match &args[0] {
        Value::Bool(true) => Ok(Value::Unit),
        Value::Bool(false) => Err("assertion failed".to_string()),
        other => Err(format!(
            "assert expects bool, got {:?}",
            builtin_type_of(std::slice::from_ref(other))?
        )),
    }
}

fn builtin_assert_eq(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("assert_eq expects 2 arguments, got {}", args.len()));
    }

    if args[0] == args[1] {
        Ok(Value::Unit)
    } else {
        Err(format!(
            "assertion failed: `{:?}` != `{:?}`",
            args[0], args[1]
        ))
    }
}

fn builtin_panic(args: &[Value]) -> Result<Value, String> {
    let message = if args.is_empty() {
        "explicit panic".to_string()
    } else {
        args.iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(" ")
    };

    Err(format!("panic: {}", message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_prelude_creates_environment() {
        let env = Environment::with_prelude();

        // Check that built-in functions are defined
        assert!(env.contains("print"));
        assert!(env.contains("println"));
        assert!(env.contains("type_of"));
        assert!(env.contains("dbg"));
        assert!(env.contains("assert"));
        assert!(env.contains("assert_eq"));
        assert!(env.contains("panic"));
    }

    #[test]
    fn test_load_prelude_adds_builtins() {
        let mut env = Environment::new();
        assert!(!env.contains("print"));

        env.load_prelude();

        assert!(env.contains("print"));
        assert!(env.contains("println"));
        assert!(env.contains("type_of"));
        assert!(env.contains("dbg"));
        assert!(env.contains("assert"));
        assert!(env.contains("assert_eq"));
        assert!(env.contains("panic"));
    }

    #[test]
    fn test_builtin_print_no_args() {
        let result = builtin_print(&[]);
        assert_eq!(result, Ok(Value::Unit));
    }

    #[test]
    fn test_builtin_print_single_arg() {
        let result = builtin_print(&[Value::I64(42)]);
        assert_eq!(result, Ok(Value::Unit));
    }

    #[test]
    fn test_builtin_print_multiple_args() {
        let result = builtin_print(&[Value::I64(1), Value::string("hello"), Value::Bool(true)]);
        assert_eq!(result, Ok(Value::Unit));
    }

    #[test]
    fn test_builtin_println_no_args() {
        let result = builtin_println(&[]);
        assert_eq!(result, Ok(Value::Unit));
    }

    #[test]
    fn test_builtin_println_with_args() {
        let result = builtin_println(&[Value::I64(42), Value::string("test")]);
        assert_eq!(result, Ok(Value::Unit));
    }

    #[test]
    fn test_builtin_type_of_primitives() {
        assert_eq!(
            builtin_type_of(&[Value::Unit]).unwrap(),
            Value::string("()")
        );
        assert_eq!(
            builtin_type_of(&[Value::Bool(true)]).unwrap(),
            Value::string("bool")
        );
        assert_eq!(
            builtin_type_of(&[Value::Char('a')]).unwrap(),
            Value::string("char")
        );
        assert_eq!(
            builtin_type_of(&[Value::I8(1)]).unwrap(),
            Value::string("i8")
        );
        assert_eq!(
            builtin_type_of(&[Value::I16(1)]).unwrap(),
            Value::string("i16")
        );
        assert_eq!(
            builtin_type_of(&[Value::I32(1)]).unwrap(),
            Value::string("i32")
        );
        assert_eq!(
            builtin_type_of(&[Value::I64(1)]).unwrap(),
            Value::string("i64")
        );
        assert_eq!(
            builtin_type_of(&[Value::I128(1)]).unwrap(),
            Value::string("i128")
        );
        assert_eq!(
            builtin_type_of(&[Value::Isize(1)]).unwrap(),
            Value::string("isize")
        );
        assert_eq!(
            builtin_type_of(&[Value::U8(1)]).unwrap(),
            Value::string("u8")
        );
        assert_eq!(
            builtin_type_of(&[Value::U16(1)]).unwrap(),
            Value::string("u16")
        );
        assert_eq!(
            builtin_type_of(&[Value::U32(1)]).unwrap(),
            Value::string("u32")
        );
        assert_eq!(
            builtin_type_of(&[Value::U64(1)]).unwrap(),
            Value::string("u64")
        );
        assert_eq!(
            builtin_type_of(&[Value::U128(1)]).unwrap(),
            Value::string("u128")
        );
        assert_eq!(
            builtin_type_of(&[Value::Usize(1)]).unwrap(),
            Value::string("usize")
        );
        assert_eq!(
            builtin_type_of(&[Value::F32(1.0)]).unwrap(),
            Value::string("f32")
        );
        assert_eq!(
            builtin_type_of(&[Value::F64(1.0)]).unwrap(),
            Value::string("f64")
        );
    }

    #[test]
    fn test_builtin_type_of_collections() {
        assert_eq!(
            builtin_type_of(&[Value::string("hi")]).unwrap(),
            Value::string("String")
        );
        assert_eq!(
            builtin_type_of(&[Value::Vec(Arc::new(vec![]))]).unwrap(),
            Value::string("Vec")
        );
        assert_eq!(
            builtin_type_of(&[Value::Tuple(Arc::new(vec![]))]).unwrap(),
            Value::string("tuple")
        );
        assert_eq!(
            builtin_type_of(&[Value::Array(Arc::new(vec![]))]).unwrap(),
            Value::string("array")
        );
    }

    #[test]
    fn test_builtin_type_of_wrong_arity() {
        let result = builtin_type_of(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        let result = builtin_type_of(&[Value::I64(1), Value::I64(2)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_builtin_dbg_returns_value() {
        let value = Value::I64(42);
        let result = builtin_dbg(&[value.clone()]);
        assert_eq!(result, Ok(value));
    }

    #[test]
    fn test_builtin_dbg_wrong_arity() {
        let result = builtin_dbg(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        let result = builtin_dbg(&[Value::I64(1), Value::I64(2)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_builtin_assert_true() {
        let result = builtin_assert(&[Value::Bool(true)]);
        assert_eq!(result, Ok(Value::Unit));
    }

    #[test]
    fn test_builtin_assert_false() {
        let result = builtin_assert(&[Value::Bool(false)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("assertion failed"));
    }

    #[test]
    fn test_builtin_assert_non_bool() {
        let result = builtin_assert(&[Value::I64(42)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects bool"));
    }

    #[test]
    fn test_builtin_assert_wrong_arity() {
        let result = builtin_assert(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        let result = builtin_assert(&[Value::Bool(true), Value::Bool(true)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_builtin_assert_eq_equal() {
        let result = builtin_assert_eq(&[Value::I64(42), Value::I64(42)]);
        assert_eq!(result, Ok(Value::Unit));
    }

    #[test]
    fn test_builtin_assert_eq_not_equal() {
        let result = builtin_assert_eq(&[Value::I64(42), Value::I64(43)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("assertion failed"));
    }

    #[test]
    fn test_builtin_assert_eq_different_types() {
        let result = builtin_assert_eq(&[Value::I64(42), Value::string("42")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("assertion failed"));
    }

    #[test]
    fn test_builtin_assert_eq_wrong_arity() {
        let result = builtin_assert_eq(&[Value::I64(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));

        let result = builtin_assert_eq(&[Value::I64(1), Value::I64(2), Value::I64(3)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));
    }

    #[test]
    fn test_builtin_panic_no_args() {
        let result = builtin_panic(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("explicit panic"));
    }

    #[test]
    fn test_builtin_panic_with_message() {
        let result = builtin_panic(&[Value::string("something went wrong")]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("panic:"));
        assert!(err.contains("something went wrong"));
    }

    #[test]
    fn test_builtin_panic_with_multiple_args() {
        let result = builtin_panic(&[Value::string("error"), Value::I64(42)]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("panic:"));
        assert!(err.contains("error"));
        assert!(err.contains("42"));
    }
}
