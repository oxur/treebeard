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
