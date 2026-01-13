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
#[derive(Clone)]
pub enum MacroBody {
    /// Native Rust function (for built-in macros)
    Native(Arc<dyn Fn(&[syn::Item]) -> Result<Vec<syn::Item>, String> + Send + Sync>),

    /// Template-based (quasiquote) - Stage 3.2
    ///
    /// Uses the template system to construct AST with placeholders that get
    /// filled in during macro expansion.
    Template(crate::template::Template),

    /// User-defined (defmacro) - Stage 3.3
    ///
    /// Stores a user-defined macro function that transforms AST at expansion time.
    /// The function takes arguments (as Values) and returns a Template or AST Value.
    UserDefined(Arc<dyn Fn(&[crate::Value]) -> Result<crate::Value, String> + Send + Sync>),
}

impl fmt::Debug for MacroBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacroBody::Native(_) => write!(f, "Native(<fn>)"),
            MacroBody::Template(template) => write!(f, "Template({:?})", template),
            MacroBody::UserDefined(_) => write!(f, "UserDefined(<fn>)"),
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
    /// use treebeard::{MacroEnvironment, MacroBody, MacroDefinition, Template, TemplateNode, Value};
    ///
    /// let mut env = MacroEnvironment::new();
    /// let template = Template::new(TemplateNode::literal(Value::string("placeholder")));
    /// let body = MacroBody::Template(template);
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

    /// Call a macro with the given arguments and return the expanded result.
    ///
    /// This method handles all three macro body types:
    /// - Native: Calls the native Rust function (expects syn::Item arguments)
    /// - Template: Expands the template with bindings from arguments
    /// - UserDefined: Calls the user-defined function with Value arguments
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the macro to call
    /// * `args` - Arguments to pass to the macro
    ///
    /// # Returns
    ///
    /// Returns the expanded result as a `Value`, or an error message.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The macro is not found
    /// - The macro expansion fails
    /// - Arguments are invalid for the macro type
    pub fn expand_macro(&self, name: &str, args: &[crate::Value]) -> Result<crate::Value, String> {
        let macro_def = self
            .get_macro(name)
            .ok_or_else(|| format!("Macro '{}' not found", name))?;

        match &macro_def.body {
            MacroBody::Native(_) => {
                // Native macros expect syn::Item arguments
                // This is a simplified implementation - full implementation would
                // convert Values to syn::Items
                Err(format!(
                    "Native macro '{}' cannot be expanded with Value arguments",
                    name
                ))
            }
            MacroBody::Template(template) => {
                // Template macros expect arguments to match template parameters
                if args.len() != macro_def.params.len() {
                    return Err(format!(
                        "Macro '{}' expects {} arguments, got {}",
                        name,
                        macro_def.params.len(),
                        args.len()
                    ));
                }

                // Build bindings from parameters and arguments
                let mut bindings = crate::template::TemplateBindings::new();
                for (param, arg) in macro_def.params.iter().zip(args.iter()) {
                    bindings.bind(param.clone(), arg.clone());
                }

                // Expand the template
                template
                    .expand(&bindings)
                    .map_err(|e| format!("Template expansion failed: {}", e))
            }
            MacroBody::UserDefined(func) => {
                // User-defined macros are called directly with Value arguments
                func(args).map_err(|e| format!("User-defined macro '{}' failed: {}", name, e))
            }
        }
    }

    /// Define a user-defined macro with a closure.
    ///
    /// This is a convenience method for creating UserDefined macros.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the macro
    /// * `params` - Parameter names
    /// * `func` - The macro transformation function
    ///
    /// # Example
    ///
    /// ```rust
    /// use treebeard::{MacroEnvironment, Value};
    /// use std::sync::Arc;
    ///
    /// let mut env = MacroEnvironment::new();
    ///
    /// // Define a simple macro that returns its first argument
    /// env.define_user_macro(
    ///     "identity",
    ///     vec!["x".to_string()],
    ///     Arc::new(|args| {
    ///         if args.is_empty() {
    ///             Err("identity requires one argument".to_string())
    ///         } else {
    ///             Ok(args[0].clone())
    ///         }
    ///     }),
    /// );
    ///
    /// // Expand the macro
    /// let result = env.expand_macro("identity", &[Value::I64(42)]).unwrap();
    /// assert_eq!(result, Value::I64(42));
    /// ```
    pub fn define_user_macro(
        &mut self,
        name: impl Into<String>,
        params: Vec<String>,
        func: Arc<dyn Fn(&[crate::Value]) -> Result<crate::Value, String> + Send + Sync>,
    ) {
        let name_string = name.into();
        let body = MacroBody::UserDefined(func);
        let macro_def = MacroDefinition::new(name_string, params, body);
        self.define_macro(macro_def);
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
    use crate::template::{Template, TemplateNode};
    use crate::Value;

    /// Helper to create a simple template for testing
    fn test_template(name: &str) -> MacroBody {
        let node = TemplateNode::literal(Value::string(name));
        MacroBody::Template(Template::new(node))
    }

    #[test]
    fn test_macro_environment_creation() {
        let env = MacroEnvironment::new();
        assert!(env.is_empty());
        assert_eq!(env.len(), 0);
    }

    #[test]
    fn test_define_and_lookup_macro() {
        let mut env = MacroEnvironment::new();
        let body = test_template("test");
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

        let body1 = test_template("first");
        let macro1 = MacroDefinition::new("test".to_string(), vec![], body1);
        env.define_macro(macro1);

        let body2 = test_template("second");
        let macro2 = MacroDefinition::new("test".to_string(), vec!["x".to_string()], body2);
        env.define_macro(macro2);

        assert_eq!(env.len(), 1); // Same name, so only one entry
        let found = env.get_macro("test");
        assert_eq!(found.unwrap().params.len(), 1); // Second definition
    }

    #[test]
    fn test_macro_environment_with_parent() {
        let mut parent = MacroEnvironment::new();
        let body1 = test_template("parent_macro");
        let macro1 = MacroDefinition::new("parent_mac".to_string(), vec![], body1);
        parent.define_macro(macro1);

        let mut child = MacroEnvironment::with_parent(parent);
        let body2 = test_template("child_macro");
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

        let body1 = test_template("m1");
        env.define_macro(MacroDefinition::new("macro1".to_string(), vec![], body1));

        let body2 = test_template("m2");
        env.define_macro(MacroDefinition::new("macro2".to_string(), vec![], body2));

        let names = env.macro_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"macro1"));
        assert!(names.contains(&"macro2"));
    }

    #[test]
    fn test_clear_macros() {
        let mut env = MacroEnvironment::new();

        let body = test_template("test");
        env.define_macro(MacroDefinition::new("test".to_string(), vec![], body));

        assert_eq!(env.len(), 1);
        env.clear();
        assert_eq!(env.len(), 0);
        assert!(env.is_empty());
    }

    #[test]
    fn test_clone_without_parent() {
        let mut parent = MacroEnvironment::new();
        let body = test_template("parent");
        parent.define_macro(MacroDefinition::new("parent_mac".to_string(), vec![], body));

        let child = MacroEnvironment::with_parent(parent);
        let cloned = child.clone_without_parent();

        // Cloned should not have parent
        assert!(!cloned.has_macro("parent_mac"));
        assert_eq!(cloned.len(), 0);
    }

    #[test]
    fn test_macro_definition_expansion_count() {
        let body = test_template("test");
        let mut macro_def = MacroDefinition::new("test".to_string(), vec![], body);

        assert_eq!(macro_def.expansion_count, 0);
        macro_def.record_expansion();
        assert_eq!(macro_def.expansion_count, 1);
        macro_def.record_expansion();
        assert_eq!(macro_def.expansion_count, 2);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // STAGE 3.3: DEFMACRO TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_define_user_macro() {
        let mut env = MacroEnvironment::new();

        // Define a simple identity macro
        env.define_user_macro(
            "identity",
            vec!["x".to_string()],
            Arc::new(|args| {
                if args.is_empty() {
                    Err("identity requires one argument".to_string())
                } else {
                    Ok(args[0].clone())
                }
            }),
        );

        assert!(env.has_macro("identity"));
        let macro_def = env.get_macro("identity").unwrap();
        assert_eq!(macro_def.name, "identity");
        assert_eq!(macro_def.params.len(), 1);
    }

    #[test]
    fn test_expand_user_macro_simple() {
        let mut env = MacroEnvironment::new();

        // Define identity macro
        env.define_user_macro(
            "identity",
            vec!["x".to_string()],
            Arc::new(|args| Ok(args[0].clone())),
        );

        // Expand it
        let result = env.expand_macro("identity", &[Value::I64(42)]).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_expand_user_macro_with_transformation() {
        let mut env = MacroEnvironment::new();

        // Define a macro that doubles its argument
        env.define_user_macro(
            "double",
            vec!["x".to_string()],
            Arc::new(|args| match &args[0] {
                Value::I64(n) => Ok(Value::I64(n * 2)),
                _ => Err("double requires integer argument".to_string()),
            }),
        );

        // Expand it
        let result = env.expand_macro("double", &[Value::I64(21)]).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_expand_user_macro_error() {
        let mut env = MacroEnvironment::new();

        // Define a macro that always fails
        env.define_user_macro(
            "fail",
            vec![],
            Arc::new(|_| Err("This macro always fails".to_string())),
        );

        // Try to expand it
        let result = env.expand_macro("fail", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("always fails"));
    }

    #[test]
    fn test_expand_macro_not_found() {
        let env = MacroEnvironment::new();
        let result = env.expand_macro("nonexistent", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_expand_template_macro() {
        let mut env = MacroEnvironment::new();

        // Define a template macro: `(list ,x ,y)`
        let template = Template::new(TemplateNode::list(vec![
            TemplateNode::literal(Value::string("list")),
            TemplateNode::unquote("x"),
            TemplateNode::unquote("y"),
        ]));

        let macro_def = MacroDefinition::new(
            "make_list".to_string(),
            vec!["x".to_string(), "y".to_string()],
            MacroBody::Template(template),
        );
        env.define_macro(macro_def);

        // Expand it
        let result = env
            .expand_macro("make_list", &[Value::I64(1), Value::I64(2)])
            .unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], Value::string("list"));
                assert_eq!(v[1], Value::I64(1));
                assert_eq!(v[2], Value::I64(2));
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_expand_template_macro_arity_mismatch() {
        let mut env = MacroEnvironment::new();

        // Define a template macro with 2 parameters
        let template = Template::new(TemplateNode::unquote("x"));
        let macro_def = MacroDefinition::new(
            "test".to_string(),
            vec!["x".to_string(), "y".to_string()],
            MacroBody::Template(template),
        );
        env.define_macro(macro_def);

        // Try to expand with wrong number of arguments
        let result = env.expand_macro("test", &[Value::I64(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));
    }

    #[test]
    fn test_expand_template_macro_with_splice() {
        let mut env = MacroEnvironment::new();

        // Define a template macro: `(progn ,@body)`
        let template = Template::new(TemplateNode::list(vec![
            TemplateNode::literal(Value::string("progn")),
            TemplateNode::splice("body"),
        ]));

        let macro_def = MacroDefinition::new(
            "progn".to_string(),
            vec!["body".to_string()],
            MacroBody::Template(template),
        );
        env.define_macro(macro_def);

        // Expand it
        let body = Value::vec(vec![Value::string("stmt1"), Value::string("stmt2")]);
        let result = env.expand_macro("progn", &[body]).unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3); // progn, stmt1, stmt2
                assert_eq!(v[0], Value::string("progn"));
                assert_eq!(v[1], Value::string("stmt1"));
                assert_eq!(v[2], Value::string("stmt2"));
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_user_macro_returns_template() {
        let mut env = MacroEnvironment::new();

        // Define a user macro that constructs and returns a template
        env.define_user_macro(
            "when",
            vec!["test".to_string(), "body".to_string()],
            Arc::new(|args| {
                // Construct: (if test body)
                Ok(Value::vec(vec![
                    Value::string("if"),
                    args[0].clone(),
                    args[1].clone(),
                ]))
            }),
        );

        // Expand it
        let result = env
            .expand_macro("when", &[Value::Bool(true), Value::string("action")])
            .unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], Value::string("if"));
                assert_eq!(v[1], Value::Bool(true));
                assert_eq!(v[2], Value::string("action"));
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_user_macro_variadic() {
        let mut env = MacroEnvironment::new();

        // Define a variadic macro that takes any number of arguments
        env.define_user_macro(
            "list",
            vec![], // No fixed params - accepts any number
            Arc::new(|args| Ok(Value::vec(args.to_vec()))),
        );

        // Expand with different numbers of arguments
        let result1 = env.expand_macro("list", &[]).unwrap();
        match result1 {
            Value::Vec(v) => assert_eq!(v.len(), 0),
            _ => panic!("Expected empty Vec"),
        }

        let result2 = env
            .expand_macro("list", &[Value::I64(1), Value::I64(2), Value::I64(3)])
            .unwrap();
        match result2 {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], Value::I64(1));
                assert_eq!(v[1], Value::I64(2));
                assert_eq!(v[2], Value::I64(3));
            }
            _ => panic!("Expected Vec with 3 elements"),
        }
    }
}
