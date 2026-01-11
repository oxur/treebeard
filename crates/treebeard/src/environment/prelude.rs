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
