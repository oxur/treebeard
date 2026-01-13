//! Evaluation context configuration

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Configuration and state for evaluation.
///
/// This is passed through all evaluation calls and controls
/// behavior like recursion limits and interruption.
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Maximum call depth (stack overflow protection)
    pub max_call_depth: usize,

    /// Interrupt flag - set to true to abort evaluation
    pub interrupt: Arc<AtomicBool>,

    /// Whether to trace evaluation (for debugging)
    pub trace: bool,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            max_call_depth: 1000,
            interrupt: Arc::new(AtomicBool::new(false)),
            trace: false,
        }
    }
}

impl EvalContext {
    /// Create a new context with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with a custom call depth limit.
    pub fn with_max_call_depth(max_depth: usize) -> Self {
        Self {
            max_call_depth: max_depth,
            ..Default::default()
        }
    }

    /// Check if evaluation has been interrupted.
    pub fn is_interrupted(&self) -> bool {
        self.interrupt.load(Ordering::Relaxed)
    }

    /// Request interruption of evaluation.
    pub fn interrupt(&self) {
        self.interrupt.store(true, Ordering::Relaxed);
    }

    /// Reset the interrupt flag.
    pub fn reset_interrupt(&self) {
        self.interrupt.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_context() {
        let ctx = EvalContext::default();
        assert_eq!(ctx.max_call_depth, 1000);
        assert!(!ctx.is_interrupted());
        assert!(!ctx.trace);
    }

    #[test]
    fn test_new_context() {
        let ctx = EvalContext::new();
        assert_eq!(ctx.max_call_depth, 1000);
        assert!(!ctx.is_interrupted());
    }

    #[test]
    fn test_with_max_call_depth() {
        let ctx = EvalContext::with_max_call_depth(500);
        assert_eq!(ctx.max_call_depth, 500);
        assert!(!ctx.is_interrupted());
    }

    #[test]
    fn test_interrupt_and_check() {
        let ctx = EvalContext::new();
        assert!(!ctx.is_interrupted());

        ctx.interrupt();
        assert!(ctx.is_interrupted());
    }

    #[test]
    fn test_reset_interrupt() {
        let ctx = EvalContext::new();
        ctx.interrupt();
        assert!(ctx.is_interrupted());

        ctx.reset_interrupt();
        assert!(!ctx.is_interrupted());
    }

    #[test]
    fn test_clone_shares_interrupt() {
        let ctx1 = EvalContext::new();
        let ctx2 = ctx1.clone();

        ctx1.interrupt();
        // Both contexts share the same interrupt flag
        assert!(ctx1.is_interrupted());
        assert!(ctx2.is_interrupted());

        ctx2.reset_interrupt();
        assert!(!ctx1.is_interrupted());
        assert!(!ctx2.is_interrupted());
    }

    #[test]
    fn test_trace_flag() {
        let mut ctx = EvalContext::new();
        assert!(!ctx.trace);

        ctx.trace = true;
        assert!(ctx.trace);
    }
}
