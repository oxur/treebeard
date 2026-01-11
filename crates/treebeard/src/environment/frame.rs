//! RAII scope guard for automatic frame cleanup

use super::Environment;

/// RAII guard that automatically pops a frame when dropped.
///
/// # Example
///
/// ```
/// use treebeard::{Environment, Value};
///
/// let mut env = Environment::new();
/// env.define("x", Value::I64(1));
///
/// {
///     let mut guard = env.scope_guard();
///     guard.define("y", Value::I64(2));
///     // y is visible here
/// }
/// // _guard dropped, frame popped, y is gone
/// assert!(!env.contains("y"));
/// assert!(env.contains("x"));
/// ```
pub struct ScopeGuard<'a> {
    env: &'a mut Environment,
}

impl Environment {
    /// Create a scope guard that pushes a frame now and pops it on drop.
    pub fn scope_guard(&mut self) -> ScopeGuard<'_> {
        self.push_frame();
        ScopeGuard { env: self }
    }
}

impl<'a> Drop for ScopeGuard<'a> {
    fn drop(&mut self) {
        self.env.pop_frame();
    }
}

impl<'a> std::ops::Deref for ScopeGuard<'a> {
    type Target = Environment;

    fn deref(&self) -> &Self::Target {
        self.env
    }
}

impl<'a> std::ops::DerefMut for ScopeGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.env
    }
}
