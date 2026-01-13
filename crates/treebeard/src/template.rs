//! Template system for quasiquote-based macro expansion
//!
//! This module implements the template system that enables Lisp-style quasiquote
//! for AST construction. Templates allow mixing literal AST structure with
//! dynamically evaluated expressions.
//!
//! # Architecture
//!
//! ```text
//! Template → [Expand] → syn AST
//!    ↑
//!    └─ TemplateNode (literal | unquote | splice)
//! ```
//!
//! # Quasiquote Semantics
//!
//! - **Quasiquote (`)**: Creates a template where most structure is literal
//! - **Unquote (,)**: Evaluates expression and substitutes single value
//! - **Unquote-splicing (,@)**: Evaluates expression and splices sequence
//!
//! # Example (Conceptual)
//!
//! ```text
//! Template: `(if ,test (progn ,@body))`
//! With: test = true, body = [stmt1, stmt2]
//! Result: (if true (progn stmt1 stmt2))
//! ```
//!
//! # Phase
//!
//! This is part of Phase 3: Macro System (Stage 3.2)

use crate::{EvalError, Value};
use std::fmt;
use std::sync::Arc;

/// A template for constructing `syn` AST with placeholders.
///
/// Templates represent AST structures where some parts are literal (fixed)
/// and other parts are placeholders that get filled in during expansion.
#[derive(Clone)]
pub struct Template {
    /// Root node of the template
    pub root: TemplateNode,

    /// Template metadata (for debugging/error reporting)
    pub metadata: TemplateMetadata,
}

impl Template {
    /// Create a new template with a root node.
    pub fn new(root: TemplateNode) -> Self {
        Self {
            root,
            metadata: TemplateMetadata::default(),
        }
    }

    /// Create a template with metadata.
    pub fn with_metadata(root: TemplateNode, metadata: TemplateMetadata) -> Self {
        Self { root, metadata }
    }

    /// Expand the template by substituting placeholders with values.
    ///
    /// # Arguments
    ///
    /// * `bindings` - Map of placeholder names to their values
    ///
    /// # Returns
    ///
    /// Returns the expanded AST as a `Value` (which can contain `syn::Item` structures).
    ///
    /// # Errors
    ///
    /// Returns `EvalError` if:
    /// - A placeholder is not found in bindings
    /// - A value cannot be coerced to the required AST type
    /// - Splicing is used in an invalid context
    pub fn expand(&self, bindings: &TemplateBindings) -> Result<Value, EvalError> {
        self.root.expand(bindings)
    }
}

impl fmt::Debug for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Template {{ root: {:?} }}", self.root)
    }
}

/// Metadata about a template (for debugging and error reporting).
#[derive(Clone, Debug, Default)]
pub struct TemplateMetadata {
    /// Template source location (optional)
    pub source: Option<String>,

    /// Line number where template was defined
    pub line: Option<usize>,

    /// Macro name that owns this template (optional)
    pub macro_name: Option<String>,
}

impl TemplateMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source location.
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the line number.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the macro name.
    pub fn with_macro_name(mut self, name: String) -> Self {
        self.macro_name = Some(name);
        self
    }
}

/// A node in a template tree.
///
/// Each node represents either a literal AST structure, a placeholder for
/// a single value (unquote), or a placeholder for a sequence (unquote-splicing).
#[derive(Clone)]
pub enum TemplateNode {
    /// Literal value - included as-is in the output
    Literal(Value),

    /// Unquote - substitute a single value
    ///
    /// The string is the placeholder name to look up in bindings.
    /// Example: `,test` → looks up "test" in bindings
    Unquote(String),

    /// Unquote-splicing - substitute and splice a sequence
    ///
    /// The string is the placeholder name to look up in bindings.
    /// The value must be a sequence (Vec, Array, etc.) that gets spliced in.
    /// Example: `,@body` → looks up "body" and splices its elements
    UnquoteSplicing(String),

    /// List of template nodes
    ///
    /// Used to represent sequences like function bodies, argument lists, etc.
    /// During expansion, this becomes a Vec<Value>.
    List(Vec<TemplateNode>),

    /// Quoted item - wraps a syn::Item directly
    ///
    /// This is used when the frontend has already parsed to syn AST
    /// and wants to include it literally in a template.
    Item(Arc<syn::Item>),
}

impl TemplateNode {
    /// Create a literal template node.
    pub fn literal(value: Value) -> Self {
        TemplateNode::Literal(value)
    }

    /// Create an unquote placeholder.
    pub fn unquote(name: impl Into<String>) -> Self {
        TemplateNode::Unquote(name.into())
    }

    /// Create an unquote-splicing placeholder.
    pub fn splice(name: impl Into<String>) -> Self {
        TemplateNode::UnquoteSplicing(name.into())
    }

    /// Create a list template node.
    pub fn list(nodes: Vec<TemplateNode>) -> Self {
        TemplateNode::List(nodes)
    }

    /// Create an item template node.
    pub fn item(item: syn::Item) -> Self {
        TemplateNode::Item(Arc::new(item))
    }

    /// Expand this node using the provided bindings.
    ///
    /// # Returns
    ///
    /// Returns a `Value` representing the expanded node.
    ///
    /// # Errors
    ///
    /// Returns `EvalError` if expansion fails.
    pub fn expand(&self, bindings: &TemplateBindings) -> Result<Value, EvalError> {
        match self {
            TemplateNode::Literal(value) => Ok(value.clone()),

            TemplateNode::Unquote(name) => {
                bindings.get(name).ok_or_else(|| EvalError::TemplateError {
                    message: format!("Unquote placeholder '{}' not found in bindings", name),
                    span: None,
                })
            }

            TemplateNode::UnquoteSplicing(_name) => {
                // Splicing cannot be expanded at the top level - it only makes sense
                // within a List context where elements can be spliced in
                Err(EvalError::TemplateError {
                    message: "Unquote-splicing cannot appear at top level".to_string(),
                    span: None,
                })
            }

            TemplateNode::List(nodes) => {
                let mut result = Vec::new();

                for node in nodes {
                    match node {
                        TemplateNode::UnquoteSplicing(name) => {
                            // Look up the value
                            let value = bindings.get(name).ok_or_else(|| {
                                EvalError::TemplateError {
                                    message: format!(
                                        "Unquote-splicing placeholder '{}' not found in bindings",
                                        name
                                    ),
                                    span: None,
                                }
                            })?;

                            // The value must be a sequence - splice its elements
                            let elements = value.as_sequence()?;
                            result.extend(elements);
                        }
                        _ => {
                            // Regular node - just expand and add
                            let expanded = node.expand(bindings)?;
                            result.push(expanded);
                        }
                    }
                }

                Ok(Value::vec(result))
            }

            TemplateNode::Item(item) => {
                // Items are wrapped in a special value type
                // This is a placeholder - in practice, frontends will need to
                // convert syn::Item to appropriate Value representations
                Ok(Value::String(Arc::new(format!("{:?}", item))))
            }
        }
    }
}

impl fmt::Debug for TemplateNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateNode::Literal(v) => write!(f, "Literal({:?})", v),
            TemplateNode::Unquote(name) => write!(f, "Unquote({})", name),
            TemplateNode::UnquoteSplicing(name) => write!(f, "Splice({})", name),
            TemplateNode::List(nodes) => write!(f, "List({:?})", nodes),
            TemplateNode::Item(_) => write!(f, "Item(<syn>)"),
        }
    }
}

/// Bindings for template expansion.
///
/// Maps placeholder names to their values during template expansion.
#[derive(Clone, Debug, Default)]
pub struct TemplateBindings {
    bindings: indexmap::IndexMap<String, Value>,
}

impl TemplateBindings {
    /// Create new empty bindings.
    pub fn new() -> Self {
        Self {
            bindings: indexmap::IndexMap::new(),
        }
    }

    /// Add a binding.
    pub fn bind(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.insert(name.into(), value);
    }

    /// Create bindings with a single entry.
    pub fn single(name: impl Into<String>, value: Value) -> Self {
        let mut bindings = Self::new();
        bindings.bind(name, value);
        bindings
    }

    /// Get a binding by name.
    pub fn get(&self, name: &str) -> Option<Value> {
        self.bindings.get(name).cloned()
    }

    /// Check if a binding exists.
    pub fn has(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Get all binding names.
    pub fn names(&self) -> Vec<&str> {
        self.bindings.keys().map(|s| s.as_str()).collect()
    }

    /// Number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if bindings are empty.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl FromIterator<(String, Value)> for TemplateBindings {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let mut bindings = Self::new();
        for (name, value) in iter {
            bindings.bind(name, value);
        }
        bindings
    }
}

/// Extension trait for Value to support template operations.
trait ValueExt {
    /// Try to get this value as a sequence for splicing.
    fn as_sequence(&self) -> Result<Vec<Value>, EvalError>;
}

impl ValueExt for Value {
    fn as_sequence(&self) -> Result<Vec<Value>, EvalError> {
        match self {
            Value::Vec(v) => Ok(v.iter().cloned().collect()),
            Value::Array(a) => Ok(a.iter().cloned().collect()),
            Value::Tuple(t) => Ok(t.to_vec()),
            _ => Err(EvalError::TemplateError {
                message: format!(
                    "Cannot splice non-sequence value (got {:?})",
                    std::mem::discriminant(self)
                ),
                span: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_creation() {
        let node = TemplateNode::literal(Value::I64(42));
        let template = Template::new(node);
        assert!(template.metadata.source.is_none());
    }

    #[test]
    fn test_template_with_metadata() {
        let node = TemplateNode::literal(Value::Bool(true));
        let metadata = TemplateMetadata::new()
            .with_source("test.oxr".to_string())
            .with_line(10)
            .with_macro_name("test_macro".to_string());
        let template = Template::with_metadata(node, metadata);

        assert_eq!(template.metadata.source, Some("test.oxr".to_string()));
        assert_eq!(template.metadata.line, Some(10));
        assert_eq!(
            template.metadata.macro_name,
            Some("test_macro".to_string())
        );
    }

    #[test]
    fn test_template_node_literal() {
        let node = TemplateNode::literal(Value::string("hello"));
        let bindings = TemplateBindings::new();
        let result = node.expand(&bindings).unwrap();
        assert_eq!(result, Value::string("hello"));
    }

    #[test]
    fn test_template_node_unquote() {
        let node = TemplateNode::unquote("x");
        let bindings = TemplateBindings::single("x", Value::I64(42));
        let result = node.expand(&bindings).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_template_node_unquote_missing() {
        let node = TemplateNode::unquote("missing");
        let bindings = TemplateBindings::new();
        let result = node.expand(&bindings);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not found in bindings"));
    }

    #[test]
    fn test_template_node_splice_at_top_level_fails() {
        let node = TemplateNode::splice("items");
        let bindings = TemplateBindings::single("items", Value::vec(vec![Value::I64(1)]));
        let result = node.expand(&bindings);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot appear at top level"));
    }

    #[test]
    fn test_template_node_list_simple() {
        let nodes = vec![
            TemplateNode::literal(Value::I64(1)),
            TemplateNode::literal(Value::I64(2)),
            TemplateNode::literal(Value::I64(3)),
        ];
        let node = TemplateNode::list(nodes);
        let bindings = TemplateBindings::new();
        let result = node.expand(&bindings).unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], Value::I64(1));
                assert_eq!(v[1], Value::I64(2));
                assert_eq!(v[2], Value::I64(3));
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_template_node_list_with_unquote() {
        let nodes = vec![
            TemplateNode::literal(Value::I64(1)),
            TemplateNode::unquote("x"),
            TemplateNode::literal(Value::I64(3)),
        ];
        let node = TemplateNode::list(nodes);
        let bindings = TemplateBindings::single("x", Value::I64(42));
        let result = node.expand(&bindings).unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], Value::I64(1));
                assert_eq!(v[1], Value::I64(42));
                assert_eq!(v[2], Value::I64(3));
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_template_node_list_with_splice() {
        let nodes = vec![
            TemplateNode::literal(Value::I64(1)),
            TemplateNode::splice("items"),
            TemplateNode::literal(Value::I64(4)),
        ];
        let node = TemplateNode::list(nodes);

        let items = Value::vec(vec![Value::I64(2), Value::I64(3)]);
        let bindings = TemplateBindings::single("items", items);
        let result = node.expand(&bindings).unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 4); // 1, [2, 3 spliced], 4
                assert_eq!(v[0], Value::I64(1));
                assert_eq!(v[1], Value::I64(2));
                assert_eq!(v[2], Value::I64(3));
                assert_eq!(v[3], Value::I64(4));
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_template_node_list_splice_empty_vec() {
        let nodes = vec![
            TemplateNode::literal(Value::I64(1)),
            TemplateNode::splice("items"),
            TemplateNode::literal(Value::I64(2)),
        ];
        let node = TemplateNode::list(nodes);

        let items = Value::vec(vec![]);
        let bindings = TemplateBindings::single("items", items);
        let result = node.expand(&bindings).unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 2); // 1, [], 2
                assert_eq!(v[0], Value::I64(1));
                assert_eq!(v[1], Value::I64(2));
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_template_node_list_splice_missing_binding() {
        let nodes = vec![TemplateNode::splice("missing")];
        let node = TemplateNode::list(nodes);
        let bindings = TemplateBindings::new();
        let result = node.expand(&bindings);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_node_list_splice_non_sequence() {
        let nodes = vec![TemplateNode::splice("x")];
        let node = TemplateNode::list(nodes);
        let bindings = TemplateBindings::single("x", Value::I64(42));
        let result = node.expand(&bindings);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot splice non-sequence"));
    }

    #[test]
    fn test_template_bindings_single() {
        let bindings = TemplateBindings::single("x", Value::I64(42));
        assert_eq!(bindings.get("x"), Some(Value::I64(42)));
        assert_eq!(bindings.len(), 1);
    }

    #[test]
    fn test_template_bindings_multiple() {
        let mut bindings = TemplateBindings::new();
        bindings.bind("x", Value::I64(1));
        bindings.bind("y", Value::I64(2));
        bindings.bind("z", Value::I64(3));

        assert_eq!(bindings.get("x"), Some(Value::I64(1)));
        assert_eq!(bindings.get("y"), Some(Value::I64(2)));
        assert_eq!(bindings.get("z"), Some(Value::I64(3)));
        assert_eq!(bindings.len(), 3);
    }

    #[test]
    fn test_template_bindings_has() {
        let bindings = TemplateBindings::single("x", Value::I64(42));
        assert!(bindings.has("x"));
        assert!(!bindings.has("y"));
    }

    #[test]
    fn test_template_bindings_names() {
        let mut bindings = TemplateBindings::new();
        bindings.bind("a", Value::I64(1));
        bindings.bind("b", Value::I64(2));

        let names = bindings.names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_template_bindings_from_iterator() {
        let items = vec![
            ("x".to_string(), Value::I64(1)),
            ("y".to_string(), Value::I64(2)),
        ];
        let bindings: TemplateBindings = items.into_iter().collect();

        assert_eq!(bindings.get("x"), Some(Value::I64(1)));
        assert_eq!(bindings.get("y"), Some(Value::I64(2)));
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_template_full_expansion() {
        // Simulate: `(if ,test (progn ,@body))`
        // With: test = true, body = [stmt1, stmt2]

        let template = Template::new(TemplateNode::list(vec![
            TemplateNode::literal(Value::string("if")),
            TemplateNode::unquote("test"),
            TemplateNode::list(vec![
                TemplateNode::literal(Value::string("progn")),
                TemplateNode::splice("body"),
            ]),
        ]));

        let mut bindings = TemplateBindings::new();
        bindings.bind("test", Value::Bool(true));
        bindings.bind(
            "body",
            Value::vec(vec![Value::string("stmt1"), Value::string("stmt2")]),
        );

        let result = template.expand(&bindings).unwrap();

        match result {
            Value::Vec(v) => {
                assert_eq!(v.len(), 3); // if, true, (progn stmt1 stmt2)
                assert_eq!(v[0], Value::string("if"));
                assert_eq!(v[1], Value::Bool(true));

                match &v[2] {
                    Value::Vec(body) => {
                        assert_eq!(body.len(), 3); // progn, stmt1, stmt2
                        assert_eq!(body[0], Value::string("progn"));
                        assert_eq!(body[1], Value::string("stmt1"));
                        assert_eq!(body[2], Value::string("stmt2"));
                    }
                    _ => panic!("Expected Vec for body, got {:?}", v[2]),
                }
            }
            _ => panic!("Expected Vec, got {:?}", result),
        }
    }

    #[test]
    fn test_value_as_sequence_vec() {
        let v = Value::vec(vec![Value::I64(1), Value::I64(2)]);
        let seq = v.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);
    }

    #[test]
    fn test_value_as_sequence_array() {
        let v = Value::array(vec![Value::I64(1), Value::I64(2)]);
        let seq = v.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);
    }

    #[test]
    fn test_value_as_sequence_tuple() {
        let v = Value::tuple(vec![Value::I64(1), Value::I64(2)]);
        let seq = v.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);
    }

    #[test]
    fn test_value_as_sequence_non_sequence_fails() {
        let v = Value::I64(42);
        let result = v.as_sequence();
        assert!(result.is_err());
    }
}
