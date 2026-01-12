//! Control flow mechanism for break/continue/return

use crate::Value;

/// Control flow signal for non-local jumps.
///
/// When `break` or `continue` is evaluated, it doesn't return a normal
/// `Result<Value, EvalError>`. Instead, it returns an `Err(EvalError::ControlFlow(...))`
/// that propagates up until caught by the enclosing loop.
#[derive(Debug, Clone)]
pub enum ControlFlow {
    /// Break out of a loop, optionally with a value.
    /// The Option<String> is the loop label (e.g., `break 'outer`).
    Break {
        /// Value to return from the loop
        value: Value,
        /// Optional loop label
        label: Option<String>,
    },

    /// Continue to next iteration of a loop.
    Continue {
        /// Optional loop label
        label: Option<String>,
    },

    /// Return from a function with a value.
    /// (Used in Stage 1.5, but defined here for completeness)
    Return {
        /// Value to return from the function
        value: Value,
    },
}

impl ControlFlow {
    /// Create a break with a value.
    pub fn break_with(value: Value) -> Self {
        ControlFlow::Break { value, label: None }
    }

    /// Create a break with unit value.
    pub fn break_unit() -> Self {
        ControlFlow::Break {
            value: Value::Unit,
            label: None,
        }
    }

    /// Create a labeled break.
    pub fn break_labeled(value: Value, label: String) -> Self {
        ControlFlow::Break {
            value,
            label: Some(label),
        }
    }

    /// Create a continue.
    pub fn continue_loop() -> Self {
        ControlFlow::Continue { label: None }
    }

    /// Create a labeled continue.
    pub fn continue_labeled(label: String) -> Self {
        ControlFlow::Continue { label: Some(label) }
    }

    /// Create a return.
    pub fn return_value(value: Value) -> Self {
        ControlFlow::Return { value }
    }

    /// Check if this control flow matches a label.
    /// None label matches any loop, Some(l) matches only that label.
    pub fn matches_label(&self, loop_label: Option<&str>) -> bool {
        match self {
            ControlFlow::Break { label, .. } | ControlFlow::Continue { label } => {
                match (label, loop_label) {
                    (None, _) => true,              // Unlabeled matches any
                    (Some(l), Some(ll)) => l == ll, // Labels must match
                    (Some(_), None) => false,       // Labeled doesn't match unlabeled
                }
            }
            ControlFlow::Return { .. } => false, // Return never matches a loop
        }
    }
}
