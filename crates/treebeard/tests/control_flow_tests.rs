//! Comprehensive tests for Stage 1.4 control flow features

use treebeard::*;

fn eval(src: &str) -> std::result::Result<Value, EvalError> {
    let expr: syn::Expr = syn::parse_str(src).expect("parse failed");
    let mut env = Environment::new();
    let ctx = EvalContext::default();
    let result = expr.eval(&mut env, &ctx);

    // Convert stray ControlFlow errors to appropriate errors
    match result {
        Err(EvalError::ControlFlow(cf)) => match cf {
            ControlFlow::Break { .. } => Err(EvalError::BreakOutsideLoop { span: None }),
            ControlFlow::Continue { .. } => Err(EvalError::ContinueOutsideLoop { span: None }),
            ControlFlow::Return { .. } => Err(EvalError::ReturnOutsideFunction { span: None }),
        },
        other => other,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// If Expression Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_if_true_branch() {
    let result = eval("if true { 42 } else { 0 }");
    assert!(matches!(result, Ok(Value::I64(42))));
}

#[test]
fn test_if_false_branch() {
    let result = eval("if false { 42 } else { 0 }");
    assert!(matches!(result, Ok(Value::I64(0))));
}

#[test]
fn test_if_without_else() {
    let result = eval("if false { 42 }");
    assert!(matches!(result, Ok(Value::Unit)));
}

#[test]
fn test_if_non_bool_condition() {
    let result = eval("if 42 { 1 } else { 2 }");
    assert!(matches!(result, Err(EvalError::TypeError { .. })));
}

#[test]
fn test_if_nested() {
    let result = eval("if true { if false { 1 } else { 2 } } else { 3 }");
    assert!(matches!(result, Ok(Value::I64(2))));
}

#[test]
fn test_if_else_if_chain() {
    let result = eval("if false { 1 } else if true { 2 } else { 3 }");
    assert!(matches!(result, Ok(Value::I64(2))));
}

// ═══════════════════════════════════════════════════════════════════════
// Block Expression Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_block_single_expr() {
    let result = eval("{ 42 }");
    assert!(matches!(result, Ok(Value::I64(42))));
}

#[test]
fn test_block_multiple_stmts() {
    let result = eval("{ 1; 2; 3 }");
    assert!(matches!(result, Ok(Value::I64(3))));
}

#[test]
fn test_block_with_semicolon() {
    let result = eval("{ 42; }");
    assert!(matches!(result, Ok(Value::Unit)));
}

#[test]
fn test_block_nested() {
    let result = eval("{ { 42 } }");
    assert!(matches!(result, Ok(Value::I64(42))));
}

// ═══════════════════════════════════════════════════════════════════════
// Match Expression Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_match_literal_int() {
    let result = eval("match 42 { 42 => 1, _ => 0 }");
    assert!(matches!(result, Ok(Value::I64(1))));
}

#[test]
fn test_match_wildcard() {
    let result = eval("match 999 { 1 => 0, _ => 42 }");
    assert!(matches!(result, Ok(Value::I64(42))));
}

#[test]
fn test_match_bool() {
    let result = eval("match true { true => 1, false => 0 }");
    assert!(matches!(result, Ok(Value::I64(1))));
}

#[test]
fn test_match_char() {
    let result = eval("match 'a' { 'a' => 1, _ => 0 }");
    assert!(matches!(result, Ok(Value::I64(1))));
}

#[test]
fn test_match_non_exhaustive() {
    let result = eval("match 42 { 1 => 0, 2 => 0 }");
    assert!(matches!(result, Err(EvalError::NonExhaustiveMatch { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Loop Expression Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_loop_with_immediate_break() {
    let result = eval("loop { break; }");
    assert!(matches!(result, Ok(Value::Unit)));
}

#[test]
fn test_loop_break_with_value() {
    let result = eval("loop { break 42; }");
    assert!(matches!(result, Ok(Value::I64(42))));
}

#[test]
fn test_while_false_condition() {
    let result = eval("while false { 42 }");
    assert!(matches!(result, Ok(Value::Unit)));
}

#[test]
fn test_while_non_bool_condition() {
    let result = eval("while 42 { break; }");
    assert!(matches!(result, Err(EvalError::TypeError { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// Break/Continue Outside Loop Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_break_outside_loop_error() {
    let result = eval("break");
    assert!(matches!(result, Err(EvalError::BreakOutsideLoop { .. })));
}

#[test]
fn test_continue_outside_loop_error() {
    let result = eval("continue");
    assert!(matches!(result, Err(EvalError::ContinueOutsideLoop { .. })));
}

#[test]
fn test_break_with_value_outside_loop() {
    let result = eval("break 42");
    assert!(matches!(result, Err(EvalError::BreakOutsideLoop { .. })));
}

// ═══════════════════════════════════════════════════════════════════════
// ControlFlow Helper Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_control_flow_break_with() {
    let cf = ControlFlow::break_with(Value::I64(42));
    assert!(matches!(cf, ControlFlow::Break { .. }));
    if let ControlFlow::Break { value, label } = cf {
        assert!(matches!(value, Value::I64(42)));
        assert!(label.is_none());
    }
}

#[test]
fn test_control_flow_break_unit() {
    let cf = ControlFlow::break_unit();
    assert!(matches!(cf, ControlFlow::Break { .. }));
    if let ControlFlow::Break { value, label } = cf {
        assert!(matches!(value, Value::Unit));
        assert!(label.is_none());
    }
}

#[test]
fn test_control_flow_break_labeled() {
    let cf = ControlFlow::break_labeled(Value::I64(1), "outer".to_string());
    assert!(matches!(cf, ControlFlow::Break { .. }));
    if let ControlFlow::Break { value, label } = cf {
        assert!(matches!(value, Value::I64(1)));
        assert_eq!(label, Some("outer".to_string()));
    }
}

#[test]
fn test_control_flow_continue_loop() {
    let cf = ControlFlow::continue_loop();
    assert!(matches!(cf, ControlFlow::Continue { .. }));
    if let ControlFlow::Continue { label } = cf {
        assert!(label.is_none());
    }
}

#[test]
fn test_control_flow_continue_labeled() {
    let cf = ControlFlow::continue_labeled("inner".to_string());
    assert!(matches!(cf, ControlFlow::Continue { .. }));
    if let ControlFlow::Continue { label } = cf {
        assert_eq!(label, Some("inner".to_string()));
    }
}

#[test]
fn test_control_flow_return_value() {
    let cf = ControlFlow::return_value(Value::Bool(true));
    assert!(matches!(cf, ControlFlow::Return { .. }));
    if let ControlFlow::Return { value } = cf {
        assert!(matches!(value, Value::Bool(true)));
    }
}

#[test]
fn test_control_flow_matches_label_unlabeled() {
    let cf = ControlFlow::break_unit();
    assert!(cf.matches_label(None));
    assert!(cf.matches_label(Some("any")));
}

#[test]
fn test_control_flow_matches_label_labeled() {
    let cf = ControlFlow::break_labeled(Value::Unit, "outer".to_string());
    assert!(!cf.matches_label(None));
    assert!(cf.matches_label(Some("outer")));
    assert!(!cf.matches_label(Some("inner")));
}

#[test]
fn test_control_flow_return_never_matches() {
    let cf = ControlFlow::return_value(Value::Unit);
    assert!(!cf.matches_label(None));
    assert!(!cf.matches_label(Some("any")));
}

// ═══════════════════════════════════════════════════════════════════════
// Error Helper Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_eval_error_is_control_flow() {
    let err = EvalError::ControlFlow(ControlFlow::break_unit());
    assert!(err.is_control_flow());

    let err2 = EvalError::DivisionByZero { span: None };
    assert!(!err2.is_control_flow());
}

#[test]
fn test_eval_error_into_control_flow() {
    let cf = ControlFlow::break_with(Value::I64(42));
    let err = EvalError::ControlFlow(cf.clone());

    let extracted = err.into_control_flow();
    assert!(extracted.is_some());
}

#[test]
fn test_eval_error_into_control_flow_none() {
    let err = EvalError::DivisionByZero { span: None };
    let extracted = err.into_control_flow();
    assert!(extracted.is_none());
}
