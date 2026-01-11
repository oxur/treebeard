---
number: 5
title: "Treebeard Implementation Stages"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-01-11
updated: 2026-01-11
state: Active
supersedes: null
superseded-by: null
version: 1.0
---

# Treebeard Implementation Stages

**Companion to:** Treebeard Project Plan  
**Version:** 1.0  
**Date:** 2026-01-11  
**Status:** Planning

---

## Purpose

This document breaks down each project phase into discrete implementation stages. Each stage represents a focused unit of work suitable for a single detailed implementation document.

The stage breakdown serves several purposes:

- **Manageable scope:** Each stage can be fully specified in a single document without overwhelming context
- **Clear checkpoints:** Every stage has unambiguous completion criteria
- **Incremental progress:** Stages build on each other, allowing verification at each step
- **Parallel planning:** Stage documents can be drafted ahead of implementation

Stage documents (to be created separately) will contain detailed implementation instructions including specific `syn` types, expected behavior, test cases, and integration points.

---

## Phase 1: Core Evaluator

Build the fundamental interpretation machinery: value representation, environment management, and the recursive AST walker for expressions and statements.

The core evaluator is the heart of Treebeard. These stages progress from foundational data structures through increasingly complex expression types, culminating in function definitions and calls.

| Stage | Name | Description |
|-------|------|-------------|
| 1.1 | Value Representation | Define the `Value` enum covering Rust's runtime types (integers, floats, bools, strings, unit, tuples, structs, enums) |
| 1.2 | Environment | Implement scoped bindings with `Environment` struct supporting variable definition, lookup, and nested scopes |
| 1.3 | Basic Expressions | Evaluate literals, paths (variable references), binary operations, and unary operations |
| 1.4 | Control Flow | Implement `if`/`else`, `match` expressions, `loop`/`while`/`for`, and `break`/`continue` |
| 1.5 | Functions | Support `fn` definitions, function calls, argument passing, and `return` statements |
| 1.6 | Statements & Blocks | Handle `let` bindings, expression statements, semicolons, and block scoping; integrate all pieces |

---

## Phase 2: Frontend Trait

Define the abstraction boundary that allows multiple languages to target the Treebeard interpreter.

This phase is smaller in implementation scope but critical for architecture. The trait design determines how cleanly frontends can integrate.

| Stage | Name | Description |
|-------|------|-------------|
| 2.1 | Trait Definition | Design and implement the `LanguageFrontend` trait with parse, expand, format, and metadata methods |
| 2.2 | Rust Frontend | Implement a frontend that parses Rust source via `syn::parse_str` and `syn::parse_file` |
| 2.3 | Oxur Frontend | Integrate Oxur's existing AST Bridge as a `LanguageFrontend` implementation |

---

## Phase 3: Macro System

Implement compile-time macro expansion for Oxur, enabling syntax transformation before evaluation.

Macros are the major missing piece in Oxur (currently 0% complete). These stages follow LFE's pattern: separate macro environment, expansion before evaluation, and hygiene for safety.

| Stage | Name | Description |
|-------|------|-------------|
| 3.1 | Macro Environment | Create `MacroEnvironment` separate from runtime, supporting macro definition storage and lookup |
| 3.2 | Quasiquote | Implement quasiquote/unquote/unquote-splicing for template-based AST construction |
| 3.3 | Defmacro | Support `defmacro` form that registers syntax transformers in the macro environment |
| 3.4 | Expansion Pass | Implement the macro expansion pass that transforms AST before evaluation |
| 3.5 | Hygiene | Add gensym and hygiene mechanism to prevent unintended variable capture |

---

## Phase 4: REPL Integration

Build a polished interactive development environment on top of Oxur's existing REPL infrastructure.

Much of the REPL infrastructure exists (60% complete). These stages focus on integration with Treebeard and polishing the user experience.

| Stage | Name | Description |
|-------|------|-------------|
| 4.1 | Evaluation Loop | Connect REPL input to Treebeard evaluator; handle multi-line input and continuation prompts |
| 4.2 | Commands | Implement REPL commands (`:help`, `:type`, `:env`, `:load`, `:quit`, etc.) |
| 4.3 | Completion | Add tab completion for bound names, keywords, and commands |
| 4.4 | Output & History | Pretty-print values with configurable depth; persist command history across sessions |

---

## Phase 5: Closures and Ownership

Complete language coverage with closure support and add runtime ownership tracking for safety.

These stages tackle two distinct but related concerns: closures require proper environment capture, and ownership tracking catches common Rust-style errors at runtime.

| Stage | Name | Description |
|-------|------|-------------|
| 5.1 | Closure Values | Extend `Value` to represent closures; capture environment at definition time |
| 5.2 | Upvalue Handling | Implement by-value and by-reference capture; handle nested closures correctly |
| 5.3 | Ownership State | Add ownership tracking (Owned/Borrowed/Moved) to values; implement state transitions |
| 5.4 | Ownership Checks | Enforce ownership rules at runtime; produce clear error messages for violations |

---

## Phase 6: Compilation Escape Hatch

Enable native compilation for performance-critical code via `rustc`.

This phase provides the bridge from interpreted code to native performance. The key constraint is that compilation must never block the user.

| Stage | Name | Description |
|-------|------|-------------|
| 6.1 | Code Generation | Convert `syn` AST back to TokenStream; generate compilable Rust source |
| 6.2 | Background Compilation | Invoke `rustc` in background thread; produce `cdylib` artifacts |
| 6.3 | Dynamic Loading | Load compiled functions via `libloading`; create callable wrappers |
| 6.4 | Hot Swap | Replace interpreted functions with compiled versions transparently; implement caching |

---

## Phase 7: Crate Loading

Access the Rust ecosystem from interpreted code.

This phase opens up the entire crates.io ecosystem to the REPL, though with some type bridging constraints.

| Stage | Name | Description |
|-------|------|-------------|
| 7.1 | Cargo Integration | Invoke Cargo to fetch and build external crates as `cdylib` |
| 7.2 | Symbol Extraction | Extract function symbols from compiled crates; build callable registry |
| 7.3 | Type Bridging | Convert between `Value` and common Rust types; handle type mismatches gracefully |
| 7.4 | Require Command | Implement `(require "crate")` command; integrate with REPL environment |

---

## Summary

| Phase | Stages | Total Stages |
|-------|--------|--------------|
| Phase 1: Core Evaluator | 1.1 – 1.6 | 6 |
| Phase 2: Frontend Trait | 2.1 – 2.3 | 3 |
| Phase 3: Macro System | 3.1 – 3.5 | 5 |
| Phase 4: REPL Integration | 4.1 – 4.4 | 4 |
| Phase 5: Closures & Ownership | 5.1 – 5.4 | 4 |
| Phase 6: Compilation | 6.1 – 6.4 | 4 |
| Phase 7: Crate Loading | 7.1 – 7.4 | 4 |
| **Total** | | **30 stages** |

Each stage will have a dedicated implementation document created before development begins.
