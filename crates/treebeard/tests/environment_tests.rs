//! Environment tests

use treebeard::*;

// ═══════════════════════════════════════════════════════════════════════
// Basic Operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_new_is_empty() {
    let env = Environment::new();
    assert!(env.is_empty());
    assert_eq!(env.len(), 0);
    assert_eq!(env.depth(), 1); // Global frame
}

#[test]
fn test_environment_define_and_get() {
    let mut env = Environment::new();
    env.define("x", Value::I64(42));

    assert_eq!(env.get("x"), Some(&Value::I64(42)));
    assert_eq!(env.get("y"), None);
    assert!(env.contains("x"));
    assert!(!env.contains("y"));
}

#[test]
fn test_environment_define_multiple() {
    let mut env = Environment::new();
    env.define("a", Value::I64(1));
    env.define("b", Value::I64(2));
    env.define("c", Value::I64(3));

    assert_eq!(env.len(), 3);
    assert_eq!(env.get("a"), Some(&Value::I64(1)));
    assert_eq!(env.get("b"), Some(&Value::I64(2)));
    assert_eq!(env.get("c"), Some(&Value::I64(3)));
}

// ═══════════════════════════════════════════════════════════════════════
// Scoping and Shadowing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_push_pop_frame() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("y", Value::I64(2));

    assert_eq!(env.depth(), 2);
    assert_eq!(env.get("x"), Some(&Value::I64(1)));
    assert_eq!(env.get("y"), Some(&Value::I64(2)));

    env.pop_frame();

    assert_eq!(env.depth(), 1);
    assert_eq!(env.get("x"), Some(&Value::I64(1)));
    assert_eq!(env.get("y"), None); // y is gone
}

#[test]
fn test_environment_shadowing() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("x", Value::I64(10)); // Shadow outer x

    assert_eq!(env.get("x"), Some(&Value::I64(10))); // Gets inner x

    env.pop_frame();

    assert_eq!(env.get("x"), Some(&Value::I64(1))); // Back to outer x
}

#[test]
fn test_environment_nested_scopes() {
    let mut env = Environment::new();
    env.define("a", Value::I64(1));

    env.push_frame(); // Depth 2
    env.define("b", Value::I64(2));

    env.push_frame(); // Depth 3
    env.define("c", Value::I64(3));

    assert_eq!(env.depth(), 3);
    assert!(env.contains("a"));
    assert!(env.contains("b"));
    assert!(env.contains("c"));

    env.pop_frame(); // Back to depth 2
    assert_eq!(env.depth(), 2);
    assert!(env.contains("a"));
    assert!(env.contains("b"));
    assert!(!env.contains("c"));

    env.pop_frame(); // Back to depth 1
    assert_eq!(env.depth(), 1);
    assert!(env.contains("a"));
    assert!(!env.contains("b"));
}

#[test]
fn test_environment_cannot_pop_global_frame() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    assert_eq!(env.depth(), 1);
    env.pop_frame(); // Should do nothing
    assert_eq!(env.depth(), 1);
    assert!(env.contains("x")); // x still there
}

#[test]
fn test_environment_contains_in_current_scope() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("y", Value::I64(2));

    assert!(env.contains("x")); // In outer scope
    assert!(env.contains("y")); // In current scope
    assert!(!env.contains_in_current_scope("x")); // x is in outer scope
    assert!(env.contains_in_current_scope("y")); // y is in current scope
}

// ═══════════════════════════════════════════════════════════════════════
// Mutability
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_mutable_binding() {
    let mut env = Environment::new();
    env.define_with_mode("x", Value::I64(1), BindingMode::Mutable);

    assert_eq!(env.get("x"), Some(&Value::I64(1)));

    env.assign("x", Value::I64(42)).unwrap();
    assert_eq!(env.get("x"), Some(&Value::I64(42)));
}

#[test]
fn test_environment_immutable_binding_error() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1)); // Immutable by default

    let result = env.assign("x", Value::I64(42));
    assert!(result.is_err());

    match result {
        Err(EnvironmentError::ImmutableBinding { name, .. }) => {
            assert_eq!(name, "x");
        }
        _ => panic!("Expected ImmutableBinding error"),
    }
}

#[test]
fn test_environment_assign_undefined_error() {
    let mut env = Environment::new();

    let result = env.assign("x", Value::I64(42));
    assert!(result.is_err());

    match result {
        Err(EnvironmentError::UndefinedVariable { name }) => {
            assert_eq!(name, "x");
        }
        _ => panic!("Expected UndefinedVariable error"),
    }
}

#[test]
fn test_environment_get_mut() {
    let mut env = Environment::new();
    env.define_with_mode("x", Value::I64(1), BindingMode::Mutable);

    {
        let value = env.get_mut("x").unwrap().unwrap();
        *value = Value::I64(100);
    }

    assert_eq!(env.get("x"), Some(&Value::I64(100)));
}

// ═══════════════════════════════════════════════════════════════════════
// Call Depth Tracking
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_call_depth() {
    let mut env = Environment::with_max_call_depth(5);

    assert_eq!(env.call_depth(), 0);

    env.enter_call().unwrap();
    assert_eq!(env.call_depth(), 1);

    env.enter_call().unwrap();
    env.enter_call().unwrap();
    assert_eq!(env.call_depth(), 3);

    env.exit_call();
    assert_eq!(env.call_depth(), 2);
}

#[test]
fn test_environment_stack_overflow() {
    let mut env = Environment::with_max_call_depth(3);

    env.enter_call().unwrap(); // 1
    env.enter_call().unwrap(); // 2
    env.enter_call().unwrap(); // 3

    let result = env.enter_call(); // 4 - should fail
    assert!(result.is_err());

    match result {
        Err(EnvironmentError::StackOverflow { depth, max }) => {
            assert_eq!(depth, 3);
            assert_eq!(max, 3);
        }
        _ => panic!("Expected StackOverflow error"),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Scope Guard
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_scope_guard_basic() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    {
        let mut guard = env.scope_guard();
        guard.define("y", Value::I64(2));
        assert!(guard.contains("y"));
    } // guard dropped, frame popped

    assert!(!env.contains("y"));
    assert!(env.contains("x"));
}

#[test]
fn test_scope_guard_nested() {
    let mut env = Environment::new();
    env.define("a", Value::I64(1));

    {
        let mut guard1 = env.scope_guard();
        guard1.define("b", Value::I64(2));

        {
            let mut guard2 = guard1.scope_guard();
            guard2.define("c", Value::I64(3));
            assert!(guard2.contains("c"));
        }

        assert!(!guard1.contains("c"));
        assert!(guard1.contains("b"));
    }

    assert!(!env.contains("b"));
    assert!(env.contains("a"));
}

// ═══════════════════════════════════════════════════════════════════════
// Iteration and Inspection
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_all_names() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));
    env.define("y", Value::I64(2));

    let names = env.all_names();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert_eq!(names.len(), 2);
}

#[test]
fn test_environment_names_in_current_scope() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    env.push_frame();
    env.define("y", Value::I64(2));
    env.define("z", Value::I64(3));

    let current_names = env.names_in_current_scope();
    assert!(!current_names.contains(&"x"));
    assert!(current_names.contains(&"y"));
    assert!(current_names.contains(&"z"));
}

#[test]
fn test_environment_clear() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));
    env.push_frame();
    env.define("y", Value::I64(2));

    env.clear();

    assert!(env.is_empty());
    assert_eq!(env.depth(), 1);
    assert!(!env.contains("x"));
    assert!(!env.contains("y"));
}

// ═══════════════════════════════════════════════════════════════════════
// Prelude / Built-ins
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_with_prelude() {
    let env = Environment::with_prelude();

    assert!(env.contains("print"));
    assert!(env.contains("println"));
    assert!(env.contains("type_of"));
    assert!(env.contains("dbg"));
    assert!(env.contains("assert"));
    assert!(env.contains("assert_eq"));
    assert!(env.contains("panic"));
}

#[test]
fn test_builtin_type_of() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("type_of") {
        let result = (f.func)(&[Value::I64(42)]).unwrap();
        assert_eq!(result, Value::string("i64"));

        let result = (f.func)(&[Value::Bool(true)]).unwrap();
        assert_eq!(result, Value::string("bool"));
    } else {
        panic!("type_of not found");
    }
}

#[test]
fn test_builtin_assert_eq_pass() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert_eq") {
        let result = (f.func)(&[Value::I64(42), Value::I64(42)]);
        assert!(result.is_ok());
    } else {
        panic!("assert_eq not found");
    }
}

#[test]
fn test_builtin_assert_eq_fail() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert_eq") {
        let result = (f.func)(&[Value::I64(42), Value::I64(43)]);
        assert!(result.is_err());
    } else {
        panic!("assert_eq not found");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Additional Built-in Function Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_builtin_print() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("print") {
        let result = (f.func)(&[Value::I64(42), Value::string("hello")]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Unit);
    } else {
        panic!("print not found");
    }
}

#[test]
fn test_builtin_println() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("println") {
        let result = (f.func)(&[Value::I64(42)]);
        assert!(result.is_ok());

        // Test with no args
        let result = (f.func)(&[]);
        assert!(result.is_ok());
    } else {
        panic!("println not found");
    }
}

#[test]
fn test_builtin_type_of_all_types() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("type_of") {
        // Test all primitive types
        assert_eq!((f.func)(&[Value::Unit]).unwrap(), Value::string("()"));
        assert_eq!(
            (f.func)(&[Value::Bool(true)]).unwrap(),
            Value::string("bool")
        );
        assert_eq!(
            (f.func)(&[Value::Char('x')]).unwrap(),
            Value::string("char")
        );
        assert_eq!((f.func)(&[Value::I8(1)]).unwrap(), Value::string("i8"));
        assert_eq!((f.func)(&[Value::I16(1)]).unwrap(), Value::string("i16"));
        assert_eq!((f.func)(&[Value::I32(1)]).unwrap(), Value::string("i32"));
        assert_eq!((f.func)(&[Value::I64(1)]).unwrap(), Value::string("i64"));
        assert_eq!((f.func)(&[Value::I128(1)]).unwrap(), Value::string("i128"));
        assert_eq!(
            (f.func)(&[Value::Isize(1)]).unwrap(),
            Value::string("isize")
        );
        assert_eq!((f.func)(&[Value::U8(1)]).unwrap(), Value::string("u8"));
        assert_eq!((f.func)(&[Value::U16(1)]).unwrap(), Value::string("u16"));
        assert_eq!((f.func)(&[Value::U32(1)]).unwrap(), Value::string("u32"));
        assert_eq!((f.func)(&[Value::U64(1)]).unwrap(), Value::string("u64"));
        assert_eq!((f.func)(&[Value::U128(1)]).unwrap(), Value::string("u128"));
        assert_eq!(
            (f.func)(&[Value::Usize(1)]).unwrap(),
            Value::string("usize")
        );
        assert_eq!((f.func)(&[Value::F32(1.0)]).unwrap(), Value::string("f32"));
        assert_eq!((f.func)(&[Value::F64(1.0)]).unwrap(), Value::string("f64"));
        assert_eq!(
            (f.func)(&[Value::string("hi")]).unwrap(),
            Value::string("String")
        );
        assert_eq!(
            (f.func)(&[Value::bytes(vec![1])]).unwrap(),
            Value::string("Vec<u8>")
        );
        assert_eq!(
            (f.func)(&[Value::vec(vec![])]).unwrap(),
            Value::string("Vec")
        );
        assert_eq!(
            (f.func)(&[Value::tuple(vec![])]).unwrap(),
            Value::string("tuple")
        );
        assert_eq!(
            (f.func)(&[Value::array(vec![])]).unwrap(),
            Value::string("array")
        );

        // Test struct and enum
        let s = StructValue::new("MyStruct");
        assert_eq!(
            (f.func)(&[Value::structure(s)]).unwrap(),
            Value::string("MyStruct")
        );

        let e = EnumValue::unit("MyEnum", "Variant");
        assert_eq!(
            (f.func)(&[Value::enumeration(e)]).unwrap(),
            Value::string("MyEnum")
        );
    } else {
        panic!("type_of not found");
    }
}

#[test]
fn test_builtin_type_of_wrong_arity() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("type_of") {
        let result = (f.func)(&[]);
        assert!(result.is_err());

        let result = (f.func)(&[Value::I64(1), Value::I64(2)]);
        assert!(result.is_err());
    } else {
        panic!("type_of not found");
    }
}

#[test]
fn test_builtin_dbg() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("dbg") {
        let value = Value::I64(42);
        let result = (f.func)(&[value.clone()]).unwrap();
        assert_eq!(result, value);
    } else {
        panic!("dbg not found");
    }
}

#[test]
fn test_builtin_dbg_wrong_arity() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("dbg") {
        let result = (f.func)(&[]);
        assert!(result.is_err());
    } else {
        panic!("dbg not found");
    }
}

#[test]
fn test_builtin_assert_pass() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert") {
        let result = (f.func)(&[Value::Bool(true)]);
        assert!(result.is_ok());
    } else {
        panic!("assert not found");
    }
}

#[test]
fn test_builtin_assert_fail() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert") {
        let result = (f.func)(&[Value::Bool(false)]);
        assert!(result.is_err());
    } else {
        panic!("assert not found");
    }
}

#[test]
fn test_builtin_assert_wrong_type() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert") {
        let result = (f.func)(&[Value::I64(1)]);
        assert!(result.is_err());
    } else {
        panic!("assert not found");
    }
}

#[test]
fn test_builtin_assert_wrong_arity() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert") {
        let result = (f.func)(&[]);
        assert!(result.is_err());
    } else {
        panic!("assert not found");
    }
}

#[test]
fn test_builtin_assert_eq_wrong_arity() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("assert_eq") {
        let result = (f.func)(&[Value::I64(1)]);
        assert!(result.is_err());
    } else {
        panic!("assert_eq not found");
    }
}

#[test]
fn test_builtin_panic_with_message() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("panic") {
        let result = (f.func)(&[Value::string("error message")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("error message"));
    } else {
        panic!("panic not found");
    }
}

#[test]
fn test_builtin_panic_no_args() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("panic") {
        let result = (f.func)(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("explicit panic"));
    } else {
        panic!("panic not found");
    }
}

#[test]
fn test_builtin_panic_multiple_args() {
    let env = Environment::with_prelude();

    if let Some(Value::BuiltinFn(f)) = env.get("panic") {
        let result = (f.func)(&[Value::string("error:"), Value::I64(42)]);
        assert!(result.is_err());
    } else {
        panic!("panic not found");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Additional Coverage Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_get_binding() {
    let mut env = Environment::new();
    env.define("x", Value::I64(42));

    let binding = env.get_binding("x").unwrap();
    assert_eq!(binding.name, "x");
    assert_eq!(binding.value, Value::I64(42));
    assert!(!binding.mutable);

    assert!(env.get_binding("y").is_none());
}

#[test]
fn test_environment_is_global_scope() {
    let mut env = Environment::new();
    assert!(env.is_global_scope());

    env.push_frame();
    assert!(!env.is_global_scope());

    env.pop_frame();
    assert!(env.is_global_scope());
}

#[test]
fn test_environment_iter() {
    let mut env = Environment::new();
    env.define("a", Value::I64(1));
    env.define("b", Value::I64(2));

    let bindings: Vec<_> = env.iter().collect();
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].name, "a");
    assert_eq!(bindings[1].name, "b");
}

#[test]
fn test_environment_default() {
    let env = Environment::default();
    assert!(env.is_empty());
    assert_eq!(env.depth(), 1);
}

#[test]
fn test_environment_get_mut_immutable_error() {
    let mut env = Environment::new();
    env.define("x", Value::I64(1));

    let result = env.get_mut("x");
    assert!(result.is_err());
}

#[test]
fn test_environment_get_mut_undefined() {
    let mut env = Environment::new();

    let result = env.get_mut("x");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_environment_exit_call_at_zero() {
    let mut env = Environment::new();
    assert_eq!(env.call_depth(), 0);

    env.exit_call(); // Should not panic, should saturate at 0
    assert_eq!(env.call_depth(), 0);
}

#[test]
fn test_binding_mode_equality() {
    assert_eq!(BindingMode::Immutable, BindingMode::Immutable);
    assert_eq!(BindingMode::Mutable, BindingMode::Mutable);
    assert_eq!(BindingMode::Constant, BindingMode::Constant);
    assert_ne!(BindingMode::Immutable, BindingMode::Mutable);
}

#[test]
fn test_environment_load_prelude() {
    let mut env = Environment::new();
    assert!(!env.contains("print"));

    env.load_prelude();
    assert!(env.contains("print"));
}
