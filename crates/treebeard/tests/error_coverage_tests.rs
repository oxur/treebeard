// Comprehensive coverage tests for error module
use treebeard::error::type_name;
use treebeard::*;

// ═══════════════════════════════════════════════════════════════════════
// type_name Coverage - All Value Types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_type_name_unit() {
    assert_eq!(type_name(&Value::Unit), "()");
}

#[test]
fn test_type_name_primitives() {
    assert_eq!(type_name(&Value::Bool(true)), "bool");
    assert_eq!(type_name(&Value::Char('a')), "char");
}

#[test]
fn test_type_name_all_signed_integers() {
    assert_eq!(type_name(&Value::I8(0)), "i8");
    assert_eq!(type_name(&Value::I16(0)), "i16");
    assert_eq!(type_name(&Value::I32(0)), "i32");
    assert_eq!(type_name(&Value::I64(0)), "i64");
    assert_eq!(type_name(&Value::I128(0)), "i128");
    assert_eq!(type_name(&Value::Isize(0)), "isize");
}

#[test]
fn test_type_name_all_unsigned_integers() {
    assert_eq!(type_name(&Value::U8(0)), "u8");
    assert_eq!(type_name(&Value::U16(0)), "u16");
    assert_eq!(type_name(&Value::U32(0)), "u32");
    assert_eq!(type_name(&Value::U64(0)), "u64");
    assert_eq!(type_name(&Value::U128(0)), "u128");
    assert_eq!(type_name(&Value::Usize(0)), "usize");
}

#[test]
fn test_type_name_floats() {
    assert_eq!(type_name(&Value::F32(0.0)), "f32");
    assert_eq!(type_name(&Value::F64(0.0)), "f64");
}

#[test]
fn test_type_name_strings_and_bytes() {
    use std::sync::Arc;
    assert_eq!(type_name(&Value::from("hello")), "String");
    assert_eq!(type_name(&Value::Bytes(Arc::new(vec![1, 2, 3]))), "Vec<u8>");
}

#[test]
fn test_type_name_collections() {
    use std::sync::Arc;
    assert_eq!(type_name(&Value::from(vec![Value::I64(1)])), "Vec");
    assert_eq!(type_name(&Value::Array(Arc::new(vec![]))), "array");
    assert_eq!(
        type_name(&Value::HashMap(Arc::new(Default::default()))),
        "HashMap"
    );
    assert_eq!(type_name(&Value::Tuple(Arc::new(vec![]))), "tuple");
}

#[test]
fn test_type_name_compound_types() {
    use indexmap::IndexMap;
    use std::sync::Arc;

    let struct_val = Value::Struct(Arc::new(StructValue {
        type_name: "MyStruct".to_string(),
        fields: IndexMap::new(),
        is_tuple_struct: false,
    }));
    assert_eq!(type_name(&struct_val), "struct");

    let enum_val = Value::Enum(Arc::new(EnumValue {
        type_name: "MyEnum".to_string(),
        variant: "Variant".to_string(),
        data: EnumData::Unit,
    }));
    assert_eq!(type_name(&enum_val), "enum");
}

#[test]
fn test_type_name_option() {
    let some_val = Value::from(Some(42));
    assert_eq!(type_name(&some_val), "Option");

    let none_val: Value = Value::from(None::<i64>);
    assert_eq!(type_name(&none_val), "Option");
}

#[test]
fn test_type_name_result() {
    let ok_val: Value = Value::from(Ok::<i64, String>(42));
    assert_eq!(type_name(&ok_val), "Result");

    let err_val: Value = Value::from(Err::<i64, String>("error".to_string()));
    assert_eq!(type_name(&err_val), "Result");
}

#[test]
fn test_type_name_references() {
    let ref_val = Value::Ref(ValueRef {
        value: std::sync::Arc::new(Value::I64(42)),
        tag: 0,
    });
    assert_eq!(type_name(&ref_val), "&T");

    let refmut_val = Value::RefMut(ValueRefMut {
        value: std::sync::Arc::new(std::sync::RwLock::new(Value::I64(42))),
        tag: 0,
    });
    assert_eq!(type_name(&refmut_val), "&mut T");
}

// ═══════════════════════════════════════════════════════════════════════
// EvalError Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_error_undefined_variable() {
    let err = EvalError::UndefinedVariable {
        name: "foo".to_string(),
        span: None,
    };
    assert!(err.to_string().contains("foo"));
    assert!(err.span().is_none());
}

#[test]
fn test_eval_error_type_error() {
    let err = EvalError::TypeError {
        message: "test error".to_string(),
        span: None,
    };
    assert!(err.to_string().contains("test error"));
}

#[test]
fn test_eval_error_division_by_zero() {
    let err = EvalError::DivisionByZero { span: None };
    assert!(err.to_string().contains("division by zero"));
}

#[test]
fn test_eval_error_integer_overflow() {
    let err = EvalError::IntegerOverflow { span: None };
    assert!(err.to_string().contains("integer overflow"));
}

#[test]
fn test_eval_error_invalid_unary_operand() {
    let err = EvalError::InvalidUnaryOperand {
        op: "-".to_string(),
        operand_type: "bool".to_string(),
        span: None,
    };
    assert!(err.to_string().contains("-"));
    assert!(err.to_string().contains("bool"));
}

#[test]
fn test_eval_error_invalid_binary_operands() {
    let err = EvalError::InvalidBinaryOperands {
        op: "+".to_string(),
        left_type: "bool".to_string(),
        right_type: "i32".to_string(),
        span: None,
    };
    assert!(err.to_string().contains("+"));
    assert!(err.to_string().contains("bool"));
    assert!(err.to_string().contains("i32"));
}

#[test]
fn test_eval_error_unsupported_expr() {
    let err = EvalError::UnsupportedExpr {
        kind: "if expression".to_string(),
        span: None,
    };
    assert!(err.to_string().contains("if expression"));
}

#[test]
fn test_eval_error_unsupported_literal() {
    let err = EvalError::UnsupportedLiteral {
        kind: "C string".to_string(),
        span: None,
    };
    assert!(err.to_string().contains("C string"));
}

#[test]
fn test_eval_error_interrupted() {
    let err = EvalError::Interrupted;
    assert!(err.to_string().contains("interrupted"));
    assert!(err.span().is_none());
}

#[test]
fn test_eval_error_stack_overflow() {
    let err = EvalError::StackOverflow { max: 1000 };
    assert!(err.to_string().contains("1000"));
    assert!(err.span().is_none());
}

// ═══════════════════════════════════════════════════════════════════════
// EnvironmentError Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_environment_error_undefined_variable() {
    let err = EnvironmentError::UndefinedVariable {
        name: "foo".to_string(),
    };
    assert!(err.to_string().contains("foo"));
}

#[test]
fn test_environment_error_immutable_binding() {
    let err = EnvironmentError::ImmutableBinding {
        name: "x".to_string(),
        span: None,
    };
    assert!(err.to_string().contains("immutable"));
    assert!(err.to_string().contains("x"));
}

#[test]
fn test_environment_error_stack_overflow() {
    let err = EnvironmentError::StackOverflow {
        depth: 1001,
        max: 1000,
    };
    assert!(err.to_string().contains("1001"));
    assert!(err.to_string().contains("1000"));
}

#[test]
fn test_environment_error_constant_redefinition() {
    let err = EnvironmentError::ConstantRedefinition {
        name: "MY_CONST".to_string(),
    };
    assert!(err.to_string().contains("MY_CONST"));
}

// ═══════════════════════════════════════════════════════════════════════
// Error From Conversion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_error_from_environment_error() {
    let env_err = EnvironmentError::UndefinedVariable {
        name: "x".to_string(),
    };
    let eval_err: EvalError = env_err.into();
    assert!(matches!(eval_err, EvalError::Environment(_)));
}
