//! Pattern matching logic

use crate::{Environment, EvalError, Value};
use proc_macro2::Span;

/// Result of pattern matching: bindings to add to environment.
pub type MatchBindings = Vec<(String, Value, bool)>; // (name, value, mutable)

/// Match a value against a pattern.
///
/// Returns `Ok(Some(bindings))` if the pattern matches,
/// `Ok(None)` if it doesn't match,
/// `Err(...)` if there's an error.
pub fn match_pattern(
    pattern: &syn::Pat,
    value: &Value,
    _span: Option<Span>,
) -> Result<Option<MatchBindings>, EvalError> {
    match pattern {
        // Wildcard: matches anything, no bindings
        syn::Pat::Wild(_) => Ok(Some(vec![])),

        // Identifier: matches anything, binds the value
        syn::Pat::Ident(pat_ident) => {
            let name = pat_ident.ident.to_string();
            let mutable = pat_ident.mutability.is_some();

            // Check for @ pattern (e.g., `x @ 1..=5`)
            if let Some((_, subpat)) = &pat_ident.subpat {
                // Must also match the subpattern
                if let Some(mut bindings) = match_pattern(subpat, value, None)? {
                    bindings.push((name, value.clone(), mutable));
                    Ok(Some(bindings))
                } else {
                    Ok(None)
                }
            } else {
                Ok(Some(vec![(name, value.clone(), mutable)]))
            }
        }

        // Literal pattern: matches exact value
        syn::Pat::Lit(pat_lit) => {
            let lit_value = crate::eval::literal::eval_lit(&pat_lit.lit)?;
            if value == &lit_value {
                Ok(Some(vec![]))
            } else {
                Ok(None)
            }
        }

        // Or pattern: try each alternative
        syn::Pat::Or(pat_or) => {
            for case in &pat_or.cases {
                if let Some(bindings) = match_pattern(case, value, None)? {
                    return Ok(Some(bindings));
                }
            }
            Ok(None)
        }

        // Tuple pattern: match each element
        syn::Pat::Tuple(pat_tuple) => match value {
            Value::Tuple(elements) => {
                if pat_tuple.elems.len() != elements.len() {
                    return Ok(None);
                }
                let mut all_bindings = vec![];
                for (pat, val) in pat_tuple.elems.iter().zip(elements.iter()) {
                    if let Some(bindings) = match_pattern(pat, val, None)? {
                        all_bindings.extend(bindings);
                    } else {
                        return Ok(None);
                    }
                }
                Ok(Some(all_bindings))
            }
            _ => Ok(None),
        },

        // Struct pattern: match fields
        syn::Pat::Struct(pat_struct) => match value {
            Value::Struct(s) => {
                // Check type name matches (simplified - just check last segment)
                let pat_type = pat_struct
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                if s.type_name != pat_type {
                    return Ok(None);
                }

                let mut all_bindings = vec![];
                for field_pat in &pat_struct.fields {
                    let field_name = match &field_pat.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(idx) => idx.index.to_string(),
                    };

                    let field_value = s.fields.get(&field_name).cloned().unwrap_or(Value::Unit);

                    if let Some(bindings) = match_pattern(&field_pat.pat, &field_value, None)? {
                        all_bindings.extend(bindings);
                    } else {
                        return Ok(None);
                    }
                }

                // Handle `..` rest pattern
                // (We don't need to do anything special - just ignore unmatched fields)

                Ok(Some(all_bindings))
            }
            _ => Ok(None),
        },

        // TupleStruct pattern (e.g., Some(x))
        syn::Pat::TupleStruct(pat_ts) => match value {
            Value::Enum(e) => {
                // Check variant name matches
                let pat_variant = pat_ts
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                if e.variant != pat_variant {
                    return Ok(None);
                }

                // Match inner data
                match &e.data {
                    crate::EnumData::Tuple(elements) => {
                        if pat_ts.elems.len() != elements.len() {
                            return Ok(None);
                        }
                        let mut all_bindings = vec![];
                        for (pat, val) in pat_ts.elems.iter().zip(elements.iter()) {
                            if let Some(bindings) = match_pattern(pat, val, None)? {
                                all_bindings.extend(bindings);
                            } else {
                                return Ok(None);
                            }
                        }
                        Ok(Some(all_bindings))
                    }
                    _ => Ok(None),
                }
            }
            Value::Option(opt) => {
                let pat_variant = pat_ts
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                match (pat_variant.as_str(), opt.as_ref()) {
                    ("Some", Some(inner)) if pat_ts.elems.len() == 1 => {
                        match_pattern(&pat_ts.elems[0], inner, None)
                    }
                    ("None", None) if pat_ts.elems.is_empty() => Ok(Some(vec![])),
                    _ => Ok(None),
                }
            }
            Value::Result(res) => {
                let pat_variant = pat_ts
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                match (pat_variant.as_str(), res.as_ref()) {
                    ("Ok", Ok(inner)) if pat_ts.elems.len() == 1 => {
                        match_pattern(&pat_ts.elems[0], inner, None)
                    }
                    ("Err", Err(inner)) if pat_ts.elems.len() == 1 => {
                        match_pattern(&pat_ts.elems[0], inner, None)
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        },

        // Path pattern (e.g., None, MyEnum::Variant)
        syn::Pat::Path(pat_path) => {
            let variant = pat_path
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();

            match value {
                Value::Option(opt) if opt.is_none() && variant == "None" => Ok(Some(vec![])),
                Value::Enum(e) if e.variant == variant => match &e.data {
                    crate::EnumData::Unit => Ok(Some(vec![])),
                    _ => Ok(None), // Has data but pattern doesn't expect it
                },
                _ => Ok(None),
            }
        }

        // Range pattern (e.g., 1..=5)
        syn::Pat::Range(pat_range) => {
            // Evaluate bounds
            let start = pat_range
                .start
                .as_ref()
                .map(|e| eval_const_expr(e))
                .transpose()?;
            let end = pat_range
                .end
                .as_ref()
                .map(|e| eval_const_expr(e))
                .transpose()?;

            let in_range = match (start, end, &pat_range.limits) {
                (Some(s), Some(e), syn::RangeLimits::Closed(_)) => {
                    value_in_range_inclusive(value, &s, &e)
                }
                (Some(s), Some(e), syn::RangeLimits::HalfOpen(_)) => {
                    value_in_range_exclusive(value, &s, &e)
                }
                _ => {
                    return Err(EvalError::UnsupportedExpr {
                        kind: "unbounded range pattern".to_string(),
                        span: None,
                    });
                }
            };

            if in_range {
                Ok(Some(vec![]))
            } else {
                Ok(None)
            }
        }

        // Reference pattern
        syn::Pat::Reference(pat_ref) => {
            // For now, just match the inner pattern against the value
            // (We're not tracking references strictly yet)
            match_pattern(&pat_ref.pat, value, None)
        }

        // Rest pattern (..) - handled by parent patterns
        syn::Pat::Rest(_) => Ok(Some(vec![])),

        // Slice pattern
        syn::Pat::Slice(pat_slice) => match value {
            Value::Vec(elements) | Value::Array(elements) => {
                // Check for rest pattern
                let has_rest = pat_slice
                    .elems
                    .iter()
                    .any(|p| matches!(p, syn::Pat::Rest(_)));

                if has_rest {
                    // Complex slice matching with ..
                    match_slice_with_rest(&pat_slice.elems, elements)
                } else {
                    // Simple: exact length match
                    if pat_slice.elems.len() != elements.len() {
                        return Ok(None);
                    }
                    let mut all_bindings = vec![];
                    for (pat, val) in pat_slice.elems.iter().zip(elements.iter()) {
                        if let Some(bindings) = match_pattern(pat, val, None)? {
                            all_bindings.extend(bindings);
                        } else {
                            return Ok(None);
                        }
                    }
                    Ok(Some(all_bindings))
                }
            }
            _ => Ok(None),
        },

        // Const pattern (named constant)
        syn::Pat::Const(_) => Err(EvalError::UnsupportedExpr {
            kind: "const pattern".to_string(),
            span: None,
        }),

        // Macro pattern
        syn::Pat::Macro(_) => Err(EvalError::UnsupportedExpr {
            kind: "macro pattern".to_string(),
            span: None,
        }),

        // Paren pattern - unwrap
        syn::Pat::Paren(pat) => match_pattern(&pat.pat, value, None),

        // Type pattern (x: Type)
        syn::Pat::Type(pat_type) => {
            // Just match the inner pattern, ignore type annotation
            match_pattern(&pat_type.pat, value, None)
        }

        // Verbatim pattern
        syn::Pat::Verbatim(_) => Err(EvalError::UnsupportedExpr {
            kind: "verbatim pattern".to_string(),
            span: None,
        }),

        _ => Err(EvalError::UnsupportedExpr {
            kind: "unknown pattern".to_string(),
            span: None,
        }),
    }
}

/// Evaluate a constant expression (for range patterns).
fn eval_const_expr(expr: &syn::Expr) -> Result<Value, EvalError> {
    // Only handle literals and negated literals for now
    match expr {
        syn::Expr::Lit(lit) => crate::eval::literal::eval_lit(&lit.lit),
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            let inner = eval_const_expr(&unary.expr)?;
            crate::eval::unary::eval_neg(inner, None)
        }
        _ => Err(EvalError::UnsupportedExpr {
            kind: "non-constant in range pattern".to_string(),
            span: None,
        }),
    }
}

/// Check if value is in inclusive range [start, end].
fn value_in_range_inclusive(value: &Value, start: &Value, end: &Value) -> bool {
    match (value, start, end) {
        (Value::I64(v), Value::I64(s), Value::I64(e)) => *v >= *s && *v <= *e,
        (Value::I32(v), Value::I32(s), Value::I32(e)) => *v >= *s && *v <= *e,
        (Value::U64(v), Value::U64(s), Value::U64(e)) => *v >= *s && *v <= *e,
        (Value::U32(v), Value::U32(s), Value::U32(e)) => *v >= *s && *v <= *e,
        (Value::Char(v), Value::Char(s), Value::Char(e)) => *v >= *s && *v <= *e,
        _ => false,
    }
}

/// Check if value is in exclusive range [start, end).
fn value_in_range_exclusive(value: &Value, start: &Value, end: &Value) -> bool {
    match (value, start, end) {
        (Value::I64(v), Value::I64(s), Value::I64(e)) => *v >= *s && *v < *e,
        (Value::I32(v), Value::I32(s), Value::I32(e)) => *v >= *s && *v < *e,
        (Value::U64(v), Value::U64(s), Value::U64(e)) => *v >= *s && *v < *e,
        (Value::U32(v), Value::U32(s), Value::U32(e)) => *v >= *s && *v < *e,
        (Value::Char(v), Value::Char(s), Value::Char(e)) => *v >= *s && *v < *e,
        _ => false,
    }
}

/// Match a slice pattern with rest (..).
fn match_slice_with_rest(
    patterns: &syn::punctuated::Punctuated<syn::Pat, syn::Token![,]>,
    elements: &[Value],
) -> Result<Option<MatchBindings>, EvalError> {
    // Find the rest pattern position
    let rest_pos = patterns
        .iter()
        .position(|p| matches!(p, syn::Pat::Rest(_)))
        .unwrap();

    let before_rest = &patterns.iter().collect::<Vec<_>>()[..rest_pos];
    let after_rest = &patterns.iter().collect::<Vec<_>>()[rest_pos + 1..];

    // Need at least enough elements for patterns before and after rest
    if elements.len() < before_rest.len() + after_rest.len() {
        return Ok(None);
    }

    let mut all_bindings = vec![];

    // Match patterns before rest
    for (pat, val) in before_rest.iter().zip(elements.iter()) {
        if let Some(bindings) = match_pattern(pat, val, None)? {
            all_bindings.extend(bindings);
        } else {
            return Ok(None);
        }
    }

    // Match patterns after rest (from the end)
    let after_start = elements.len() - after_rest.len();
    for (pat, val) in after_rest.iter().zip(elements[after_start..].iter()) {
        if let Some(bindings) = match_pattern(pat, val, None)? {
            all_bindings.extend(bindings);
        } else {
            return Ok(None);
        }
    }

    Ok(Some(all_bindings))
}

/// Apply match bindings to the environment.
pub fn apply_bindings(env: &mut Environment, bindings: MatchBindings) {
    for (name, value, mutable) in bindings {
        if mutable {
            env.define_with_mode(&name, value, crate::BindingMode::Mutable);
        } else {
            env.define(&name, value);
        }
    }
}
