//! Macro environment for compile-time macro expansion
//!
//! This module implements the macro environment that stores macro definitions
//! separately from the runtime environment. This follows LFE's pattern of
//! separating compile-time and runtime concerns.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐
//! │ MacroEnvironment│     │   Environment   │
//! │  (compile-time) │     │   (runtime)     │
//! ├─────────────────┤     ├─────────────────┤
//! │ defmacro defs   │     │ defn defs       │
//! │ syntax rules    │     │ variables       │
//! │ expansions      │     │ values          │
//! └─────────────────┘     └─────────────────┘
//!         │                       │
//!         ↓                       ↓
//!   Macro Expansion         Evaluation
//! ```
//!
//! # Phase
//!
//! This is part of Phase 3: Macro System (Stage 3.1)

use indexmap::IndexMap;
use std::fmt;
use std::sync::Arc;

/// A macro definition stored in the macro environment.
///
/// Macros are functions that transform AST at compile-time. They take
/// S-expressions (or syn AST) as input and produce transformed AST as output.
#[derive(Clone)]
pub struct MacroDefinition {
    /// Name of the macro
    pub name: String,

    /// Parameter names (patterns to match)
    pub params: Vec<String>,

    /// Macro body (transformation rules)
    /// This will be expanded in Stage 3.3 (Defmacro)
    pub body: MacroBody,

    /// Number of times this macro has been expanded (for debugging)
    pub expansion_count: usize,
}

impl MacroDefinition {
    /// Create a new macro definition
    pub fn new(name: String, params: Vec<String>, body: MacroBody) -> Self {
        Self {
            name,
            params,
            body,
            expansion_count: 0,
        }
    }

    /// Increment the expansion counter
    pub fn record_expansion(&mut self) {
        self.expansion_count += 1;
    }
}

impl fmt::Debug for MacroDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MacroDefinition {{ name: {}, params: {:?}, expansions: {} }}",
            self.name, self.params, self.expansion_count
        )
    }
}

/// The body of a macro definition.
///
/// This represents the transformation rules that define what the macro does.
/// Initially this is a placeholder; full implementation comes in Stage 3.3.
#[derive(Clone)]
pub enum MacroBody {
    /// Native Rust function (for built-in macros)
    Native(Arc<dyn Fn(&[syn::Item]) -> Result<Vec<syn::Item>, String> + Send + Sync>),

    /// Template-based (quasiquote) - Stage 3.2
    /// Will store the template and expansion rules
    Template {
        /// The quasiquoted template
        template: String,
        /// Reserved for template expansion logic
        _marker: std::marker::PhantomData<()>,
    },

    /// User-defined (defmacro) - Stage 3.3
    /// Will store the macro's implementation
    UserDefined {
        /// The macro implementation (to be defined)
        implementation: String,
        /// Reserved for expansion logic
        _marker: std::marker::PhantomData<()>,
    },
}

impl fmt::Debug for MacroBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacroBody::Native(_) => write!(f, "Native(<fn>)"),
            MacroBody::Template { template, .. } => write!(f, "Template({})", template),
            MacroBody::UserDefined { implementation, .. } => {
                write!(f, "UserDefined({})", implementation)
            }
        }
    }
}

/// Macro environment for compile-time macro expansion.
///
/// This environment is separate from the runtime environment and stores
/// macro definitions. Macros are expanded before evaluation, transforming
/// the AST.
///
/// # Example
///
/// ```rust
/// use treebeard::MacroEnvironment;
///
/// let mut env = MacroEnvironment::new();
///
/// // Register a macro (Stage 3.3 will provide the full API)
/// // env.define_macro("when", ...);
///
/// // Look up a macro
/// assert!(env.get_macro("when").is_none());
/// ```
#[derive(Clone, Default)]
pub struct MacroEnvironment {
    /// Macro definitions, indexed by name
    /// Using IndexMap to preserve definition order
    macros: IndexMap<String, Arc<MacroDefinition>>,

    /// Gensym counter for generating unique symbols (Stage 3.5)
    gensym_counter: u64,

    /// Parent environment for nested scopes (Stage 3.4)
    /// Macros can be defined in local scopes
    parent: Option<Box<MacroEnvironment>>,
}

impl MacroEnvironment {
    /// Create a new empty macro environment.
    pub fn new() -> Self {
        Self {
            macros: IndexMap::new(),
            gensym_counter: 0,
            parent: None,
        }
    }

    /// Create a new macro environment with a parent.
    ///
    /// This allows for nested macro scopes where local macros shadow
    /// outer definitions.
    pub fn with_parent(parent: MacroEnvironment) -> Self {
        Self {
            macros: IndexMap::new(),
            gensym_counter: parent.gensym_counter,
            parent: Some(Box::new(parent)),
        }
    }

    /// Define a new macro in this environment.
    ///
    /// If a macro with the same name already exists, it is shadowed.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the macro
    /// * `params` - Parameter names for the macro
    /// * `body` - The macro body (transformation rules)
    ///
    /// # Example
    ///
    /// ```rust
    /// use treebeard::{MacroEnvironment, MacroBody, MacroDefinition};
    ///
    /// let mut env = MacroEnvironment::new();
    /// let body = MacroBody::Template {
    ///     template: "placeholder".to_string(),
    ///     _marker: std::marker::PhantomData,
    /// };
    /// let macro_def = MacroDefinition::new("my_macro".to_string(), vec![], body);
    /// env.define_macro(macro_def);
    ///
    /// assert!(env.get_macro("my_macro").is_some());
    /// ```
    pub fn define_macro(&mut self, macro_def: MacroDefinition) {
        let name = macro_def.name.clone();
        self.macros.insert(name, Arc::new(macro_def));
    }

    /// Look up a macro by name.
    ///
    /// Searches the current environment and then parent environments.
    ///
    /// # Returns
    ///
    /// Returns `Some(&MacroDefinition)` if found, `None` otherwise.
    pub fn get_macro(&self, name: &str) -> Option<&Arc<MacroDefinition>> {
        // Check current environment
        if let Some(macro_def) = self.macros.get(name) {
            return Some(macro_def);
        }

        // Check parent environment
        if let Some(parent) = &self.parent {
            return parent.get_macro(name);
        }

        None
    }

    /// Check if a macro is defined.
    pub fn has_macro(&self, name: &str) -> bool {
        self.get_macro(name).is_some()
    }

    /// Get all macro names defined in this environment (not including parent).
    pub fn macro_names(&self) -> Vec<&str> {
        self.macros.keys().map(|s| s.as_str()).collect()
    }

    /// Generate a unique symbol for hygiene (Stage 3.5).
    ///
    /// This is used to prevent variable capture in macro expansions.
    ///
    /// # Arguments
    ///
    /// * `base` - The base name for the generated symbol
    ///
    /// # Returns
    ///
    /// Returns a unique symbol like `base_G42` where the number is unique.
    ///
    /// # Example
    ///
    /// ```rust
    /// use treebeard::MacroEnvironment;
    ///
    /// let mut env = MacroEnvironment::new();
    /// let sym1 = env.gensym("temp");
    /// let sym2 = env.gensym("temp");
    /// assert_ne!(sym1, sym2);
    /// assert!(sym1.starts_with("temp_G"));
    /// ```
    pub fn gensym(&mut self, base: &str) -> String {
        let id = self.gensym_counter;
        self.gensym_counter += 1;
        format!("{}_G{}", base, id)
    }

    /// Reset the gensym counter (useful for testing).
    pub fn reset_gensym(&mut self) {
        self.gensym_counter = 0;
    }

    /// Get the number of macros defined (not including parent).
    pub fn len(&self) -> usize {
        self.macros.len()
    }

    /// Check if the environment is empty (not including parent).
    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    /// Clear all macros from this environment (not including parent).
    pub fn clear(&mut self) {
        self.macros.clear();
    }

    /// Clone the environment without the parent (for creating child scopes).
    pub fn clone_without_parent(&self) -> Self {
        Self {
            macros: self.macros.clone(),
            gensym_counter: self.gensym_counter,
            parent: None,
        }
    }
}

impl fmt::Debug for MacroEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MacroEnvironment {{ macros: {:?}, gensym_counter: {}, has_parent: {} }}",
            self.macro_names(),
            self.gensym_counter,
            self.parent.is_some()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_environment_creation() {
        let env = MacroEnvironment::new();
        assert!(env.is_empty());
        assert_eq!(env.len(), 0);
    }

    #[test]
    fn test_define_and_lookup_macro() {
        let mut env = MacroEnvironment::new();
        let body = MacroBody::Template {
            template: "test".to_string(),
            _marker: std::marker::PhantomData,
        };
        let macro_def = MacroDefinition::new("my_macro".to_string(), vec![], body);

        env.define_macro(macro_def);

        assert!(env.has_macro("my_macro"));
        assert_eq!(env.len(), 1);

        let found = env.get_macro("my_macro");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my_macro");
    }

    #[test]
    fn test_macro_lookup_missing() {
        let env = MacroEnvironment::new();
        assert!(!env.has_macro("nonexistent"));
        assert!(env.get_macro("nonexistent").is_none());
    }

    #[test]
    fn test_macro_shadowing() {
        let mut env = MacroEnvironment::new();

        let body1 = MacroBody::Template {
            template: "first".to_string(),
            _marker: std::marker::PhantomData,
        };
        let macro1 = MacroDefinition::new("test".to_string(), vec![], body1);
        env.define_macro(macro1);

        let body2 = MacroBody::Template {
            template: "second".to_string(),
            _marker: std::marker::PhantomData,
        };
        let macro2 = MacroDefinition::new("test".to_string(), vec!["x".to_string()], body2);
        env.define_macro(macro2);

        assert_eq!(env.len(), 1); // Same name, so only one entry
        let found = env.get_macro("test");
        assert_eq!(found.unwrap().params.len(), 1); // Second definition
    }

    #[test]
    fn test_macro_environment_with_parent() {
        let mut parent = MacroEnvironment::new();
        let body1 = MacroBody::Template {
            template: "parent_macro".to_string(),
            _marker: std::marker::PhantomData,
        };
        let macro1 = MacroDefinition::new("parent_mac".to_string(), vec![], body1);
        parent.define_macro(macro1);

        let mut child = MacroEnvironment::with_parent(parent);
        let body2 = MacroBody::Template {
            template: "child_macro".to_string(),
            _marker: std::marker::PhantomData,
        };
        let macro2 = MacroDefinition::new("child_mac".to_string(), vec![], body2);
        child.define_macro(macro2);

        // Child can see its own macro
        assert!(child.has_macro("child_mac"));
        // Child can see parent's macro
        assert!(child.has_macro("parent_mac"));
        // Child's len() only counts its own macros
        assert_eq!(child.len(), 1);
    }

    #[test]
    fn test_gensym_unique() {
        let mut env = MacroEnvironment::new();

        let sym1 = env.gensym("temp");
        let sym2 = env.gensym("temp");
        let sym3 = env.gensym("var");

        assert_ne!(sym1, sym2);
        assert_ne!(sym2, sym3);
        assert!(sym1.starts_with("temp_G"));
        assert!(sym2.starts_with("temp_G"));
        assert!(sym3.starts_with("var_G"));
    }

    #[test]
    fn test_gensym_reset() {
        let mut env = MacroEnvironment::new();

        let sym1 = env.gensym("x");
        env.reset_gensym();
        let sym2 = env.gensym("x");

        // After reset, should start from 0 again
        assert_eq!(sym1, sym2);
    }

    #[test]
    fn test_macro_names() {
        let mut env = MacroEnvironment::new();

        let body1 = MacroBody::Template {
            template: "m1".to_string(),
            _marker: std::marker::PhantomData,
        };
        env.define_macro(MacroDefinition::new("macro1".to_string(), vec![], body1));

        let body2 = MacroBody::Template {
            template: "m2".to_string(),
            _marker: std::marker::PhantomData,
        };
        env.define_macro(MacroDefinition::new("macro2".to_string(), vec![], body2));

        let names = env.macro_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"macro1"));
        assert!(names.contains(&"macro2"));
    }

    #[test]
    fn test_clear_macros() {
        let mut env = MacroEnvironment::new();

        let body = MacroBody::Template {
            template: "test".to_string(),
            _marker: std::marker::PhantomData,
        };
        env.define_macro(MacroDefinition::new("test".to_string(), vec![], body));

        assert_eq!(env.len(), 1);
        env.clear();
        assert_eq!(env.len(), 0);
        assert!(env.is_empty());
    }

    #[test]
    fn test_clone_without_parent() {
        let mut parent = MacroEnvironment::new();
        let body = MacroBody::Template {
            template: "parent".to_string(),
            _marker: std::marker::PhantomData,
        };
        parent.define_macro(MacroDefinition::new("parent_mac".to_string(), vec![], body));

        let child = MacroEnvironment::with_parent(parent);
        let cloned = child.clone_without_parent();

        // Cloned should not have parent
        assert!(!cloned.has_macro("parent_mac"));
        assert_eq!(cloned.len(), 0);
    }

    #[test]
    fn test_macro_definition_expansion_count() {
        let body = MacroBody::Template {
            template: "test".to_string(),
            _marker: std::marker::PhantomData,
        };
        let mut macro_def = MacroDefinition::new("test".to_string(), vec![], body);

        assert_eq!(macro_def.expansion_count, 0);
        macro_def.record_expansion();
        assert_eq!(macro_def.expansion_count, 1);
        macro_def.record_expansion();
        assert_eq!(macro_def.expansion_count, 2);
    }
}
