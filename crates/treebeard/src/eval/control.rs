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

impl PartialEq for ControlFlow {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                ControlFlow::Break {
                    value: v1,
                    label: l1,
                },
                ControlFlow::Break {
                    value: v2,
                    label: l2,
                },
            ) => v1 == v2 && l1 == l2,
            (ControlFlow::Continue { label: l1 }, ControlFlow::Continue { label: l2 }) => l1 == l2,
            (ControlFlow::Return { value: v1 }, ControlFlow::Return { value: v2 }) => v1 == v2,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_break_with() {
        let cf = ControlFlow::break_with(Value::I64(42));
        match cf {
            ControlFlow::Break { value, label } => {
                assert_eq!(value, Value::I64(42));
                assert_eq!(label, None);
            }
            _ => panic!("Expected Break"),
        }
    }

    #[test]
    fn test_break_unit() {
        let cf = ControlFlow::break_unit();
        match cf {
            ControlFlow::Break { value, label } => {
                assert_eq!(value, Value::Unit);
                assert_eq!(label, None);
            }
            _ => panic!("Expected Break"),
        }
    }

    #[test]
    fn test_break_labeled() {
        let cf = ControlFlow::break_labeled(Value::I64(42), "outer".to_string());
        match cf {
            ControlFlow::Break { value, label } => {
                assert_eq!(value, Value::I64(42));
                assert_eq!(label, Some("outer".to_string()));
            }
            _ => panic!("Expected Break"),
        }
    }

    #[test]
    fn test_continue_loop() {
        let cf = ControlFlow::continue_loop();
        match cf {
            ControlFlow::Continue { label } => {
                assert_eq!(label, None);
            }
            _ => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_continue_labeled() {
        let cf = ControlFlow::continue_labeled("outer".to_string());
        match cf {
            ControlFlow::Continue { label } => {
                assert_eq!(label, Some("outer".to_string()));
            }
            _ => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_return_value() {
        let cf = ControlFlow::return_value(Value::I64(42));
        match cf {
            ControlFlow::Return { value } => {
                assert_eq!(value, Value::I64(42));
            }
            _ => panic!("Expected Return"),
        }
    }

    #[test]
    fn test_matches_label_unlabeled_break() {
        let cf = ControlFlow::break_unit();
        assert!(cf.matches_label(None)); // Matches unlabeled loop
        assert!(cf.matches_label(Some("outer"))); // Matches any labeled loop
    }

    #[test]
    fn test_matches_label_labeled_break() {
        let cf = ControlFlow::break_labeled(Value::Unit, "outer".to_string());
        assert!(!cf.matches_label(None)); // Doesn't match unlabeled loop
        assert!(cf.matches_label(Some("outer"))); // Matches same label
        assert!(!cf.matches_label(Some("inner"))); // Doesn't match different label
    }

    #[test]
    fn test_matches_label_continue() {
        let cf = ControlFlow::continue_loop();
        assert!(cf.matches_label(None)); // Matches unlabeled loop
        assert!(cf.matches_label(Some("outer"))); // Matches any labeled loop

        let cf_labeled = ControlFlow::continue_labeled("outer".to_string());
        assert!(!cf_labeled.matches_label(None)); // Doesn't match unlabeled
        assert!(cf_labeled.matches_label(Some("outer"))); // Matches same label
        assert!(!cf_labeled.matches_label(Some("inner"))); // Doesn't match different label
    }

    #[test]
    fn test_matches_label_return() {
        let cf = ControlFlow::return_value(Value::I64(42));
        assert!(!cf.matches_label(None)); // Return never matches loop
        assert!(!cf.matches_label(Some("outer")));
    }

    #[test]
    fn test_partialeq_break() {
        let cf1 = ControlFlow::break_with(Value::I64(42));
        let cf2 = ControlFlow::break_with(Value::I64(42));
        let cf3 = ControlFlow::break_with(Value::I64(43));
        assert_eq!(cf1, cf2);
        assert_ne!(cf1, cf3);
    }

    #[test]
    fn test_partialeq_continue() {
        let cf1 = ControlFlow::continue_loop();
        let cf2 = ControlFlow::continue_loop();
        let cf3 = ControlFlow::continue_labeled("outer".to_string());
        assert_eq!(cf1, cf2);
        assert_ne!(cf1, cf3);
    }

    #[test]
    fn test_partialeq_return() {
        let cf1 = ControlFlow::return_value(Value::I64(42));
        let cf2 = ControlFlow::return_value(Value::I64(42));
        let cf3 = ControlFlow::return_value(Value::I64(43));
        assert_eq!(cf1, cf2);
        assert_ne!(cf1, cf3);
    }

    #[test]
    fn test_partialeq_different_types() {
        let brk = ControlFlow::break_unit();
        let cont = ControlFlow::continue_loop();
        let ret = ControlFlow::return_value(Value::Unit);
        assert_ne!(brk, cont);
        assert_ne!(brk, ret);
        assert_ne!(cont, ret);
    }
}
