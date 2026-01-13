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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn test_scope_guard_creates_frame() {
        let mut env = Environment::new();
        let initial_depth = env.depth();

        {
            let guard = env.scope_guard();
            assert_eq!(guard.depth(), initial_depth + 1);
        }
    }

    #[test]
    fn test_scope_guard_drops_frame() {
        let mut env = Environment::new();
        let initial_depth = env.depth();

        {
            let guard = env.scope_guard();
            assert_eq!(guard.depth(), initial_depth + 1);
        }
        // After drop
        assert_eq!(env.depth(), initial_depth);
    }

    #[test]
    fn test_scope_guard_isolates_variables() {
        let mut env = Environment::new();
        env.define("outer", Value::I64(1));

        {
            let mut guard = env.scope_guard();
            guard.define("inner", Value::I64(2));

            // Both should be visible inside guard
            assert!(guard.contains("outer"));
            assert!(guard.contains("inner"));
        }

        // After drop, inner should be gone
        assert!(env.contains("outer"));
        assert!(!env.contains("inner"));
    }

    #[test]
    fn test_scope_guard_deref_read() {
        let mut env = Environment::new();
        env.define("x", Value::I64(42));

        {
            let guard = env.scope_guard();
            // Test Deref - can read through guard
            assert!(guard.contains("x"));
            assert_eq!(guard.get("x"), Some(&Value::I64(42)));
        }
    }

    #[test]
    fn test_scope_guard_deref_mut_write() {
        let mut env = Environment::new();

        {
            let mut guard = env.scope_guard();
            // Test DerefMut - can write through guard
            guard.define("y", Value::I64(100));
            assert_eq!(guard.get("y"), Some(&Value::I64(100)));
        }

        // Variable defined in guard should be gone after drop
        assert!(!env.contains("y"));
    }

    #[test]
    fn test_scope_guard_nested_scopes() {
        let mut env = Environment::new();
        env.define("a", Value::I64(1));

        {
            let mut guard1 = env.scope_guard();
            guard1.define("b", Value::I64(2));

            {
                let mut guard2 = guard1.scope_guard();
                guard2.define("c", Value::I64(3));

                // All three should be visible
                assert!(guard2.contains("a"));
                assert!(guard2.contains("b"));
                assert!(guard2.contains("c"));
            }

            // After inner guard drops, c should be gone
            assert!(guard1.contains("a"));
            assert!(guard1.contains("b"));
            assert!(!guard1.contains("c"));
        }

        // After outer guard drops, only a remains
        assert!(env.contains("a"));
        assert!(!env.contains("b"));
        assert!(!env.contains("c"));
    }

    #[test]
    fn test_scope_guard_shadows_outer_variable() {
        let mut env = Environment::new();
        env.define("x", Value::I64(1));

        {
            let mut guard = env.scope_guard();
            guard.define("x", Value::I64(2));

            // Inner x shadows outer x
            assert_eq!(guard.get("x"), Some(&Value::I64(2)));
        }

        // After drop, outer x is visible again
        assert_eq!(env.get("x"), Some(&Value::I64(1)));
    }

    #[test]
    fn test_scope_guard_lookup_mutation() {
        let mut env = Environment::new();
        env.define_with_mode("x", Value::I64(10), crate::BindingMode::Mutable);

        {
            let mut guard = env.scope_guard();
            // Mutate outer variable through guard
            if let Ok(Some(val)) = guard.get_mut("x") {
                *val = Value::I64(20);
            }
            assert_eq!(guard.get("x"), Some(&Value::I64(20)));
        }

        // Mutation should persist after guard drops
        assert_eq!(env.get("x"), Some(&Value::I64(20)));
    }
}
