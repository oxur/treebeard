//! Error types for Treebeard evaluation

use proc_macro2::Span;
use thiserror::Error;

/// Main error type for Treebeard operations
#[derive(Error, Debug)]
pub enum TreebeardError {
    /// Type mismatch error
    #[error("Type error: expected {expected}, got {got}")]
    TypeError {
        /// Expected type
        expected: String,
        /// Actual type received
        got: String,
    },

    /// Value error
    #[error("Value error: {0}")]
    ValueError(String),

    /// Feature not yet implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Result type alias for Treebeard operations
pub type Result<T> = std::result::Result<T, TreebeardError>;

/// Errors that can occur during environment operations
#[derive(Error, Debug, Clone)]
pub enum EnvironmentError {
    /// Attempted to access an undefined variable
    #[error("undefined variable `{name}`")]
    UndefinedVariable {
        /// Variable name
        name: String,
    },

    /// Attempted to mutate an immutable binding
    #[error("cannot assign to immutable binding `{name}`")]
    ImmutableBinding {
        /// Binding name
        name: String,
        /// Location where binding was defined
        span: Option<Span>,
    },

    /// Call stack overflow (too much recursion)
    #[error("stack overflow: call depth {depth} exceeds maximum {max}")]
    StackOverflow {
        /// Current call depth
        depth: usize,
        /// Maximum allowed depth
        max: usize,
    },

    /// Attempted to redefine a constant
    #[error("cannot redefine constant `{name}`")]
    ConstantRedefinition {
        /// Constant name
        name: String,
    },
}

/// Errors that can occur during evaluation
#[derive(Error, Debug, Clone)]
pub enum EvalError {
    /// Undefined variable reference
    #[error("undefined variable `{name}`")]
    UndefinedVariable {
        /// Variable name
        name: String,
        /// Source span
        span: Option<Span>,
    },

    /// Type mismatch in operation
    #[error("type error: {message}")]
    TypeError {
        /// Error message
        message: String,
        /// Source span
        span: Option<Span>,
    },

    /// Division by zero
    #[error("division by zero")]
    DivisionByZero {
        /// Source span
        span: Option<Span>,
    },

    /// Integer overflow
    #[error("integer overflow")]
    IntegerOverflow {
        /// Source span
        span: Option<Span>,
    },

    /// Invalid operand for unary operator
    #[error("cannot apply `{op}` to {operand_type}")]
    InvalidUnaryOperand {
        /// Operator
        op: String,
        /// Operand type
        operand_type: String,
        /// Source span
        span: Option<Span>,
    },

    /// Invalid operands for binary operator
    #[error("cannot apply `{op}` to {left_type} and {right_type}")]
    InvalidBinaryOperands {
        /// Operator
        op: String,
        /// Left operand type
        left_type: String,
        /// Right operand type
        right_type: String,
        /// Source span
        span: Option<Span>,
    },

    /// Unsupported expression type
    #[error("unsupported expression: {kind}")]
    UnsupportedExpr {
        /// Expression kind
        kind: String,
        /// Source span
        span: Option<Span>,
    },

    /// Unsupported literal type
    #[error("unsupported literal: {kind}")]
    UnsupportedLiteral {
        /// Literal kind
        kind: String,
        /// Source span
        span: Option<Span>,
    },

    /// Evaluation was interrupted
    #[error("evaluation interrupted")]
    Interrupted,

    /// Stack overflow (too much recursion)
    #[error("stack overflow: maximum call depth ({max}) exceeded")]
    StackOverflow {
        /// Maximum allowed depth
        max: usize,
    },

    /// Control flow (break/continue/return) - not really an error,
    /// but uses the error path for propagation.
    #[error("control flow")]
    ControlFlow(crate::eval::control::ControlFlow),

    /// Break outside of loop.
    #[error("`break` outside of loop")]
    BreakOutsideLoop {
        /// Source span
        span: Option<Span>,
    },

    /// Continue outside of loop.
    #[error("`continue` outside of loop")]
    ContinueOutsideLoop {
        /// Source span
        span: Option<Span>,
    },

    /// Return outside of function.
    #[error("`return` outside of function")]
    ReturnOutsideFunction {
        /// Source span
        span: Option<Span>,
    },

    /// Non-exhaustive match.
    #[error("non-exhaustive match: `{value}` not covered")]
    NonExhaustiveMatch {
        /// Value that wasn't covered
        value: String,
        /// Source span
        span: Option<Span>,
    },

    /// Refutable pattern in irrefutable context.
    #[error("refutable pattern in local binding")]
    RefutablePattern {
        /// Pattern description
        pattern: String,
        /// Source span
        span: Option<Span>,
    },

    /// Arity mismatch in function call.
    #[error("function `{name}` expected {expected} argument(s), got {got}")]
    ArityMismatch {
        /// Expected argument count
        expected: usize,
        /// Actual argument count received
        got: usize,
        /// Function name
        name: String,
        /// Source span
        span: Option<Span>,
    },

    /// Built-in function error.
    #[error("built-in function `{name}`: {message}")]
    BuiltinError {
        /// Built-in function name
        name: String,
        /// Error message
        message: String,
        /// Source span
        span: Option<Span>,
    },

    /// Invalid assignment target.
    #[error("cannot assign to {kind}")]
    InvalidAssignTarget {
        /// Description of what was attempted
        kind: String,
        /// Source span
        span: Option<Span>,
    },

    /// Index out of bounds.
    #[error("index out of bounds: index {index} >= len {len}")]
    IndexOutOfBounds {
        /// Index that was accessed
        index: usize,
        /// Length of the collection
        len: usize,
        /// Source span
        span: Option<Span>,
    },

    /// Key not found in map.
    #[error("key not found: {key}")]
    KeyNotFound {
        /// Key that was not found
        key: String,
        /// Source span
        span: Option<Span>,
    },

    /// Field not found on struct.
    #[error("no field `{field}` on type `{type_name}`")]
    UndefinedField {
        /// Field name
        field: String,
        /// Type name
        type_name: String,
        /// Source span
        span: Option<Span>,
    },

    /// Let-else didn't diverge.
    #[error("let-else block must diverge (return, break, continue, or panic)")]
    NonDivergingLetElse {
        /// Source span
        span: Option<Span>,
    },

    /// Parse error.
    #[error("parse error: {message}")]
    ParseError {
        /// Error message
        message: String,
        /// Source span
        span: Option<Span>,
    },

    /// Template expansion error.
    #[error("template error: {message}")]
    TemplateError {
        /// Error message
        message: String,
        /// Source span
        span: Option<Span>,
    },

    /// Environment error wrapper
    #[error(transparent)]
    Environment(#[from] EnvironmentError),
}

impl EvalError {
    /// Get the source span for this error, if available.
    pub fn span(&self) -> Option<Span> {
        match self {
            EvalError::UndefinedVariable { span, .. } => *span,
            EvalError::TypeError { span, .. } => *span,
            EvalError::DivisionByZero { span } => *span,
            EvalError::IntegerOverflow { span } => *span,
            EvalError::InvalidUnaryOperand { span, .. } => *span,
            EvalError::InvalidBinaryOperands { span, .. } => *span,
            EvalError::UnsupportedExpr { span, .. } => *span,
            EvalError::UnsupportedLiteral { span, .. } => *span,
            EvalError::Interrupted => None,
            EvalError::StackOverflow { .. } => None,
            EvalError::ControlFlow(_) => None,
            EvalError::BreakOutsideLoop { span } => *span,
            EvalError::ContinueOutsideLoop { span } => *span,
            EvalError::ReturnOutsideFunction { span } => *span,
            EvalError::NonExhaustiveMatch { span, .. } => *span,
            EvalError::RefutablePattern { span, .. } => *span,
            EvalError::ArityMismatch { span, .. } => *span,
            EvalError::BuiltinError { span, .. } => *span,
            EvalError::InvalidAssignTarget { span, .. } => *span,
            EvalError::IndexOutOfBounds { span, .. } => *span,
            EvalError::KeyNotFound { span, .. } => *span,
            EvalError::UndefinedField { span, .. } => *span,
            EvalError::NonDivergingLetElse { span } => *span,
            EvalError::ParseError { span, .. } => *span,
            EvalError::TemplateError { span, .. } => *span,
            EvalError::Environment(_) => None,
        }
    }

    /// Check if this is a control flow "error" (not a real error).
    pub fn is_control_flow(&self) -> bool {
        matches!(self, EvalError::ControlFlow(_))
    }

    /// Extract control flow if this is one.
    pub fn into_control_flow(self) -> Option<crate::eval::control::ControlFlow> {
        match self {
            EvalError::ControlFlow(cf) => Some(cf),
            _ => None,
        }
    }
}

/// Helper to get a type name for error messages.
pub fn type_name(value: &crate::Value) -> &'static str {
    match value {
        crate::Value::Unit => "()",
        crate::Value::Bool(_) => "bool",
        crate::Value::Char(_) => "char",
        crate::Value::I8(_) => "i8",
        crate::Value::I16(_) => "i16",
        crate::Value::I32(_) => "i32",
        crate::Value::I64(_) => "i64",
        crate::Value::I128(_) => "i128",
        crate::Value::Isize(_) => "isize",
        crate::Value::U8(_) => "u8",
        crate::Value::U16(_) => "u16",
        crate::Value::U32(_) => "u32",
        crate::Value::U64(_) => "u64",
        crate::Value::U128(_) => "u128",
        crate::Value::Usize(_) => "usize",
        crate::Value::F32(_) => "f32",
        crate::Value::F64(_) => "f64",
        crate::Value::String(_) => "String",
        crate::Value::Bytes(_) => "Vec<u8>",
        crate::Value::Vec(_) => "Vec",
        crate::Value::Tuple(_) => "tuple",
        crate::Value::Array(_) => "array",
        crate::Value::Struct(_) => "struct",
        crate::Value::Enum(_) => "enum",
        crate::Value::HashMap(_) => "HashMap",
        crate::Value::Option(_) => "Option",
        crate::Value::Result(_) => "Result",
        crate::Value::Function(_) => "fn",
        crate::Value::Closure(_) => "closure",
        crate::Value::BuiltinFn(_) => "builtin_fn",
        crate::Value::CompiledFn(_) => "compiled_fn",
        crate::Value::Ref(_) => "&T",
        crate::Value::RefMut(_) => "&mut T",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use std::sync::Arc;

    #[test]
    fn test_treebeard_error_type_error_display() {
        let err = TreebeardError::TypeError {
            expected: "i64".to_string(),
            got: "String".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Type error"));
        assert!(msg.contains("i64"));
        assert!(msg.contains("String"));
    }

    #[test]
    fn test_treebeard_error_value_error_display() {
        let err = TreebeardError::ValueError("invalid value".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Value error"));
        assert!(msg.contains("invalid value"));
    }

    #[test]
    fn test_treebeard_error_not_implemented_display() {
        let err = TreebeardError::NotImplemented("feature X".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Not implemented"));
        assert!(msg.contains("feature X"));
    }

    #[test]
    fn test_environment_error_undefined_variable() {
        let err = EnvironmentError::UndefinedVariable {
            name: "foo".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("undefined variable"));
        assert!(msg.contains("foo"));
    }

    #[test]
    fn test_environment_error_immutable_binding() {
        let err = EnvironmentError::ImmutableBinding {
            name: "x".to_string(),
            span: None,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("cannot assign"));
        assert!(msg.contains("immutable"));
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_environment_error_stack_overflow() {
        let err = EnvironmentError::StackOverflow {
            depth: 1001,
            max: 1000,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("stack overflow"));
        assert!(msg.contains("1001"));
        assert!(msg.contains("1000"));
    }

    #[test]
    fn test_environment_error_constant_redefinition() {
        let err = EnvironmentError::ConstantRedefinition {
            name: "MAX".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("cannot redefine"));
        assert!(msg.contains("constant"));
        assert!(msg.contains("MAX"));
    }

    #[test]
    fn test_eval_error_span_extraction() {
        // Test errors with span = None
        assert!(EvalError::Interrupted.span().is_none());
        assert!(EvalError::StackOverflow { max: 100 }.span().is_none());

        // Test error with span = Some
        let span = proc_macro2::Span::call_site();
        let err = EvalError::TypeError {
            message: "test".to_string(),
            span: Some(span),
        };
        assert!(err.span().is_some());
    }

    #[test]
    fn test_eval_error_is_control_flow() {
        use crate::eval::control::ControlFlow;

        let cf_err = EvalError::ControlFlow(ControlFlow::break_unit());
        assert!(cf_err.is_control_flow());

        let other_err = EvalError::Interrupted;
        assert!(!other_err.is_control_flow());
    }

    #[test]
    fn test_eval_error_into_control_flow() {
        use crate::eval::control::ControlFlow;

        let cf = ControlFlow::break_unit();
        let err = EvalError::ControlFlow(cf);
        let extracted = err.into_control_flow();
        assert!(extracted.is_some());

        let other_err = EvalError::Interrupted;
        assert!(other_err.into_control_flow().is_none());
    }

    #[test]
    fn test_type_name_primitives() {
        assert_eq!(type_name(&Value::Unit), "()");
        assert_eq!(type_name(&Value::Bool(true)), "bool");
        assert_eq!(type_name(&Value::Char('a')), "char");
        assert_eq!(type_name(&Value::I8(1)), "i8");
        assert_eq!(type_name(&Value::I16(1)), "i16");
        assert_eq!(type_name(&Value::I32(1)), "i32");
        assert_eq!(type_name(&Value::I64(1)), "i64");
        assert_eq!(type_name(&Value::I128(1)), "i128");
        assert_eq!(type_name(&Value::Isize(1)), "isize");
        assert_eq!(type_name(&Value::U8(1)), "u8");
        assert_eq!(type_name(&Value::U16(1)), "u16");
        assert_eq!(type_name(&Value::U32(1)), "u32");
        assert_eq!(type_name(&Value::U64(1)), "u64");
        assert_eq!(type_name(&Value::U128(1)), "u128");
        assert_eq!(type_name(&Value::Usize(1)), "usize");
        assert_eq!(type_name(&Value::F32(1.0)), "f32");
        assert_eq!(type_name(&Value::F64(1.0)), "f64");
    }

    #[test]
    fn test_type_name_collections() {
        assert_eq!(type_name(&Value::string("hi")), "String");
        assert_eq!(type_name(&Value::Vec(Arc::new(vec![]))), "Vec");
        assert_eq!(type_name(&Value::Tuple(Arc::new(vec![]))), "tuple");
        assert_eq!(type_name(&Value::Array(Arc::new(vec![]))), "array");
    }

    #[test]
    fn test_eval_error_display_messages() {
        // Test various error display implementations
        let err = EvalError::DivisionByZero { span: None };
        assert!(format!("{}", err).contains("division by zero"));

        let err = EvalError::IntegerOverflow { span: None };
        assert!(format!("{}", err).contains("integer overflow"));

        let err = EvalError::BreakOutsideLoop { span: None };
        assert!(format!("{}", err).contains("break"));

        let err = EvalError::ContinueOutsideLoop { span: None };
        assert!(format!("{}", err).contains("continue"));

        let err = EvalError::ReturnOutsideFunction { span: None };
        assert!(format!("{}", err).contains("return"));
    }
}
