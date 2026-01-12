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
