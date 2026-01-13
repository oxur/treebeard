//! Macro expansion utilities for language frontends
//!
//! This module provides helper functions for implementing macro expansion
//! in language frontends. The expansion pass transforms AST by recursively
//! expanding macro invocations.
//!
//! # Architecture
//!
//! ```text
//! Source → Parse → AST-with-macros → [Expansion Pass] → AST → Eval
//! ```
//!
//! # Phase
//!
//! This is part of Phase 3: Macro System (Stage 3.4)

use crate::{MacroEnvironment, Value};

/// Result type for expansion operations.
pub type ExpansionResult<T> = std::result::Result<T, ExpansionError>;

/// Errors that can occur during macro expansion.
#[derive(Debug, Clone)]
pub enum ExpansionError {
    /// Macro not found in environment
    MacroNotFound {
        /// Name of the macro
        name: String,
    },

    /// Macro expansion failed
    ExpansionFailed {
        /// Macro name
        macro_name: String,
        /// Error message
        message: String,
    },

    /// Recursion depth exceeded
    RecursionLimitExceeded {
        /// Current depth
        depth: usize,
        /// Maximum allowed depth
        max_depth: usize,
    },

    /// Invalid macro invocation
    InvalidInvocation {
        /// Error message
        message: String,
    },
}

impl std::fmt::Display for ExpansionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpansionError::MacroNotFound { name } => {
                write!(f, "Macro '{}' not found", name)
            }
            ExpansionError::ExpansionFailed {
                macro_name,
                message,
            } => {
                write!(f, "Expansion of macro '{}' failed: {}", macro_name, message)
            }
            ExpansionError::RecursionLimitExceeded { depth, max_depth } => {
                write!(
                    f,
                    "Macro expansion recursion limit exceeded: depth {} > max {}",
                    depth, max_depth
                )
            }
            ExpansionError::InvalidInvocation { message } => {
                write!(f, "Invalid macro invocation: {}", message)
            }
        }
    }
}

impl std::error::Error for ExpansionError {}

/// Configuration for the expansion pass.
#[derive(Debug, Clone)]
pub struct ExpansionConfig {
    /// Maximum recursion depth for macro expansion
    pub max_depth: usize,

    /// Whether to expand macros recursively (deep expansion)
    pub deep: bool,

    /// Whether to collect and remove macro definitions
    /// (useful for compilation, not for REPL)
    pub collect_definitions: bool,
}

impl Default for ExpansionConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            deep: true,
            collect_definitions: false,
        }
    }
}

impl ExpansionConfig {
    /// Create a new expansion config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create config for REPL use (deep expansion, no collection).
    pub fn for_repl() -> Self {
        Self {
            max_depth: 100,
            deep: true,
            collect_definitions: false,
        }
    }

    /// Create config for compilation (collect definitions).
    pub fn for_compilation() -> Self {
        Self {
            max_depth: 100,
            deep: true,
            collect_definitions: true,
        }
    }

    /// Set the maximum recursion depth.
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set whether to perform deep (recursive) expansion.
    pub fn with_deep(mut self, deep: bool) -> Self {
        self.deep = deep;
        self
    }

    /// Set whether to collect macro definitions.
    pub fn with_collect_definitions(mut self, collect: bool) -> Self {
        self.collect_definitions = collect;
        self
    }
}

/// Context for macro expansion tracking recursion depth.
#[derive(Debug)]
pub struct ExpansionContext {
    /// Current recursion depth
    depth: usize,

    /// Configuration
    config: ExpansionConfig,
}

impl ExpansionContext {
    /// Create a new expansion context with the given config.
    pub fn new(config: ExpansionConfig) -> Self {
        Self { depth: 0, config }
    }

    /// Create a context with default configuration.
    pub fn default() -> Self {
        Self::new(ExpansionConfig::default())
    }

    /// Create a nested context (increment depth).
    pub fn nested(&self) -> ExpansionResult<Self> {
        let new_depth = self.depth + 1;
        if new_depth > self.config.max_depth {
            return Err(ExpansionError::RecursionLimitExceeded {
                depth: new_depth,
                max_depth: self.config.max_depth,
            });
        }
        Ok(Self {
            depth: new_depth,
            config: self.config.clone(),
        })
    }

    /// Get the current recursion depth.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Get the configuration.
    pub fn config(&self) -> &ExpansionConfig {
        &self.config
    }

    /// Check if deep expansion is enabled.
    pub fn is_deep(&self) -> bool {
        self.config.deep
    }
}

/// Expand a macro invocation to a Value.
///
/// This is a helper function for frontends. It:
/// 1. Looks up the macro in the environment
/// 2. Calls the macro with the given arguments
/// 3. Returns the expanded result
///
/// # Arguments
///
/// * `macro_name` - Name of the macro to expand
/// * `args` - Arguments to pass to the macro
/// * `env` - Macro environment
/// * `ctx` - Expansion context (for recursion tracking)
///
/// # Returns
///
/// Returns the expanded Value, or an error if expansion fails.
///
/// # Example
///
/// ```rust
/// use treebeard::expansion::{expand_macro_invocation, ExpansionContext, ExpansionConfig};
/// use treebeard::{MacroEnvironment, Value};
/// use std::sync::Arc;
///
/// let mut env = MacroEnvironment::new();
///
/// // Define a simple macro
/// env.define_user_macro(
///     "double",
///     vec!["x".to_string()],
///     Arc::new(|args| match &args[0] {
///         Value::I64(n) => Ok(Value::I64(n * 2)),
///         _ => Err("double requires integer".to_string()),
///     }),
/// );
///
/// // Expand it
/// let ctx = ExpansionContext::new(ExpansionConfig::default());
/// let result = expand_macro_invocation("double", &[Value::I64(21)], &env, &ctx).unwrap();
/// assert_eq!(result, Value::I64(42));
/// ```
pub fn expand_macro_invocation(
    macro_name: &str,
    args: &[Value],
    env: &MacroEnvironment,
    _ctx: &ExpansionContext,
) -> ExpansionResult<Value> {
    // Check if macro exists
    if !env.has_macro(macro_name) {
        return Err(ExpansionError::MacroNotFound {
            name: macro_name.to_string(),
        });
    }

    // Expand using the macro environment's expand_macro method
    env.expand_macro(macro_name, args)
        .map_err(|message| ExpansionError::ExpansionFailed {
            macro_name: macro_name.to_string(),
            message,
        })
}

/// Check if a given name is a macro in the environment.
///
/// This is a convenience function for frontends to quickly check
/// if something is a macro call vs a function call.
pub fn is_macro(name: &str, env: &MacroEnvironment) -> bool {
    env.has_macro(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{Template, TemplateNode};
    use std::sync::Arc;

    #[test]
    fn test_expansion_config_default() {
        let config = ExpansionConfig::default();
        assert_eq!(config.max_depth, 100);
        assert!(config.deep);
        assert!(!config.collect_definitions);
    }

    #[test]
    fn test_expansion_config_for_repl() {
        let config = ExpansionConfig::for_repl();
        assert!(config.deep);
        assert!(!config.collect_definitions);
    }

    #[test]
    fn test_expansion_config_for_compilation() {
        let config = ExpansionConfig::for_compilation();
        assert!(config.deep);
        assert!(config.collect_definitions);
    }

    #[test]
    fn test_expansion_config_builder() {
        let config = ExpansionConfig::new()
            .with_max_depth(50)
            .with_deep(false)
            .with_collect_definitions(true);

        assert_eq!(config.max_depth, 50);
        assert!(!config.deep);
        assert!(config.collect_definitions);
    }

    #[test]
    fn test_expansion_context_creation() {
        let ctx = ExpansionContext::default();
        assert_eq!(ctx.depth(), 0);
        assert!(ctx.is_deep());
    }

    #[test]
    fn test_expansion_context_nesting() {
        let ctx = ExpansionContext::default();
        let nested1 = ctx.nested().unwrap();
        assert_eq!(nested1.depth(), 1);

        let nested2 = nested1.nested().unwrap();
        assert_eq!(nested2.depth(), 2);
    }

    #[test]
    fn test_expansion_context_recursion_limit() {
        let config = ExpansionConfig::new().with_max_depth(3);
        let ctx = ExpansionContext::new(config);

        let nested1 = ctx.nested().unwrap();
        let nested2 = nested1.nested().unwrap();
        let nested3 = nested2.nested().unwrap();

        // Fourth level should fail
        let result = nested3.nested();
        assert!(result.is_err());
        match result {
            Err(ExpansionError::RecursionLimitExceeded { depth, max_depth }) => {
                assert_eq!(depth, 4);
                assert_eq!(max_depth, 3);
            }
            _ => panic!("Expected RecursionLimitExceeded error"),
        }
    }

    #[test]
    fn test_expand_macro_invocation_user_defined() {
        let mut env = MacroEnvironment::new();

        // Define a simple doubling macro
        env.define_user_macro(
            "double",
            vec!["x".to_string()],
            Arc::new(|args| match &args[0] {
                Value::I64(n) => Ok(Value::I64(n * 2)),
                _ => Err("double requires integer".to_string()),
            }),
        );

        let ctx = ExpansionContext::default();
        let result = expand_macro_invocation("double", &[Value::I64(21)], &env, &ctx).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_expand_macro_invocation_template() {
        let mut env = MacroEnvironment::new();

        // Define a template macro: (list ,x ,y)
        let template = Template::new(TemplateNode::list(vec![
            TemplateNode::literal(Value::string("list")),
            TemplateNode::unquote("x"),
            TemplateNode::unquote("y"),
        ]));

        let macro_def = crate::MacroDefinition::new(
            "make_list".to_string(),
            vec!["x".to_string(), "y".to_string()],
            crate::MacroBody::Template(template),
        );
        env.define_macro(macro_def);

        let ctx = ExpansionContext::default();
        let result = expand_macro_invocation(
            "make_list",
            &[Value::I64(1), Value::I64(2)],
            &env,
            &ctx,
        )
        .unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], Value::string("list"));
                assert_eq!(v[1], Value::I64(1));
                assert_eq!(v[2], Value::I64(2));
            }
            _ => panic!("Expected Vec"),
        }
    }

    #[test]
    fn test_expand_macro_invocation_not_found() {
        let env = MacroEnvironment::new();
        let ctx = ExpansionContext::default();
        let result = expand_macro_invocation("nonexistent", &[], &env, &ctx);

        assert!(result.is_err());
        match result {
            Err(ExpansionError::MacroNotFound { name }) => {
                assert_eq!(name, "nonexistent");
            }
            _ => panic!("Expected MacroNotFound error"),
        }
    }

    #[test]
    fn test_expand_macro_invocation_expansion_failed() {
        let mut env = MacroEnvironment::new();

        // Define a macro that always fails
        env.define_user_macro(
            "fail",
            vec![],
            Arc::new(|_| Err("This macro always fails".to_string())),
        );

        let ctx = ExpansionContext::default();
        let result = expand_macro_invocation("fail", &[], &env, &ctx);

        assert!(result.is_err());
        match result {
            Err(ExpansionError::ExpansionFailed { macro_name, message }) => {
                assert_eq!(macro_name, "fail");
                assert!(message.contains("always fails"));
            }
            _ => panic!("Expected ExpansionFailed error"),
        }
    }

    #[test]
    fn test_is_macro() {
        let mut env = MacroEnvironment::new();
        env.define_user_macro(
            "test",
            vec![],
            Arc::new(|_| Ok(Value::Unit)),
        );

        assert!(is_macro("test", &env));
        assert!(!is_macro("nonexistent", &env));
    }

    #[test]
    fn test_expansion_error_display() {
        let err = ExpansionError::MacroNotFound {
            name: "test".to_string(),
        };
        assert!(err.to_string().contains("not found"));

        let err = ExpansionError::ExpansionFailed {
            macro_name: "test".to_string(),
            message: "failed".to_string(),
        };
        assert!(err.to_string().contains("Expansion of macro"));

        let err = ExpansionError::RecursionLimitExceeded {
            depth: 10,
            max_depth: 5,
        };
        assert!(err.to_string().contains("recursion limit"));

        let err = ExpansionError::InvalidInvocation {
            message: "bad call".to_string(),
        };
        assert!(err.to_string().contains("Invalid macro invocation"));
    }
}
