---
number: 4
title: "Treebeard Project Plan"
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

# Treebeard Project Plan

**A Tree-Walking Interpreter for Rust's `syn` AST**

**Version:** 1.0  
**Date:** 2026-01-11  
**Status:** Planning

---

## Overview

Treebeard is a tree-walking interpreter that directly executes Rust's `syn` AST without invoking `rustc`. It provides immediate execution of Rust code with zero compilation delay, enabling REPL-driven development, hot code reloading, and rapid iteration for any language that can produce `syn` AST.

The project follows a phased approach, with each phase delivering standalone value while laying foundations for subsequent work.

---

## Project Phases

### Phase 1: Core Evaluator

**Duration:** 4 weeks  
**Goal:** Execute `syn` AST expressions and statements directly.

This phase establishes the fundamental interpretation loop. We build the recursive AST walker that traverses `syn` nodes and computes results, along with the value representation system and environment for variable bindings.

**Key Deliverables:**
- Value type representing all Rust runtime values
- Environment for variable and function bindings  
- Expression evaluator covering literals, binary/unary operations, blocks, control flow
- Statement evaluator for let bindings, expression statements, and function definitions
- Basic pattern matching for let bindings and match expressions

**Success Criteria:**  
Evaluate simple Rust programs including arithmetic, conditionals, loops, and function calls.

---

### Phase 2: Frontend Trait

**Duration:** 2 weeks  
**Goal:** Establish the abstraction boundary between language frontends and the interpreter.

This phase defines the `LanguageFrontend` trait that allows multiple languages to target Treebeard. Any language that can produce `syn` AST can use the interpreter, enabling Oxur (Lisp syntax), native Rust, and future DSLs to share the same execution engine.

**Key Deliverables:**
- `LanguageFrontend` trait specification
- Rust frontend implementation (parse Rust source via `syn`)
- Oxur frontend integration (leverage existing 95% AST Bridge)
- Error formatting interface for language-specific diagnostics

**Success Criteria:**  
Same interpreter core executes both Rust syntax and Oxur S-expressions.

---

### Phase 3: Macro System

**Duration:** 3 weeks  
**Goal:** Implement macro expansion for Oxur.

This phase fills the critical gap in Oxur's current implementation (0% complete). We implement a hygienic macro system that expands macros before evaluation, following LFE's pattern of separating compile-time and runtime environments.

**Key Deliverables:**
- Macro environment separate from runtime environment
- `defmacro` for defining syntax transformations
- Macro expansion pass before evaluation
- Hygiene mechanism to prevent accidental variable capture
- Quasiquote/unquote for template-based code generation

**Success Criteria:**  
Define and use macros in Oxur that generate correct `syn` AST.

---

### Phase 4: REPL Integration  

**Duration:** 2 weeks  
**Goal:** Deliver a fully functional interactive development environment.

This phase builds on Oxur's existing REPL infrastructure (60% complete) to create a polished interactive experience. The REPL becomes the primary interface for exploring and developing code.

**Key Deliverables:**
- Multi-line input handling with bracket matching
- Command system (`:help`, `:type`, `:env`, `:load`, `:quit`)
- Tab completion for bindings and keywords
- History with persistence across sessions
- Pretty-printed output with configurable depth
- Slurp command for loading file definitions

**Success Criteria:**  
Comfortable interactive development experience comparable to established REPLs.

---

### Phase 5: Closures and Ownership

**Duration:** 3 weeks  
**Goal:** Complete language coverage with closures and runtime ownership tracking.

This phase implements closures with proper environment capture and adds lightweight ownership tracking to catch common errors (use-after-move, double-move) at runtime without full borrow checking.

**Key Deliverables:**
- Closure values with captured environment
- Upvalue handling (by-value and by-reference capture)
- Ownership state tracking per value (Owned, Borrowed, Moved)
- Runtime checks for ownership violations
- Clear error messages for ownership errors

**Success Criteria:**  
Closures work correctly; obvious ownership bugs are caught at runtime with helpful messages.

---

### Phase 6: Compilation Escape Hatch

**Duration:** 3 weeks  
**Goal:** Enable native compilation for performance-critical code paths.

This phase provides the "escape hatch" to `rustc` for code that needs native performance. Functions can be explicitly compiled to native code and dynamically loaded, providing 100x speedups for hot paths while maintaining the interpreted development experience.

**Key Deliverables:**
- `compile` command to compile functions to native code
- Background compilation (non-blocking)
- Dynamic loading of compiled functions
- Transparent fallback (interpret until compilation completes)
- Caching of compiled artifacts

**Success Criteria:**  
`(compile fib)` produces 100x speedup for compute-intensive functions.

---

### Phase 7: Crate Loading

**Duration:** 2 weeks  
**Goal:** Access the Rust ecosystem from the REPL.

This phase enables using external Rust crates from interpreted code. Users can load crates, call their functions, and work with their types, bringing the entire Rust ecosystem into the interactive environment.

**Key Deliverables:**
- `require` command for loading crates
- Wrapper generation for crate functions
- Type bridging between Value and Rust types
- Symbol extraction from compiled crates
- Cargo integration for building dependencies

**Success Criteria:**  
Load and use external crates like `regex` or `serde_json` from the REPL.

---

## Timeline Summary

| Phase | Duration | Cumulative | Milestone |
|-------|----------|------------|-----------|
| Phase 1: Core Evaluator | 4 weeks | Week 4 | Basic evaluation works |
| Phase 2: Frontend Trait | 2 weeks | Week 6 | Multi-language support |
| Phase 3: Macro System | 3 weeks | Week 9 | Oxur macros work |
| Phase 4: REPL Integration | 2 weeks | Week 11 | Usable REPL |
| Phase 5: Closures + Ownership | 3 weeks | Week 14 | Full language coverage |
| Phase 6: Compilation | 3 weeks | Week 17 | Performance escape hatch |
| Phase 7: Crate Loading | 2 weeks | Week 19 | Rust ecosystem access |

**Total: ~19 weeks to full implementation**

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Language Frontends                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │    Oxur     │  │    Rust     │  │  Your DSL   │              │
│  │ S-expr → syn│  │ source→syn  │  │ syntax→syn  │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
│         └────────────────┼────────────────┘                      │
│                          │                                       │
│              LanguageFrontend Trait                              │
└──────────────────────────┼───────────────────────────────────────┘
                           │
                       syn AST
                           │
┌──────────────────────────┼───────────────────────────────────────┐
│                    Treebeard Core                                │
│                          │                                       │
│  ┌───────────────────────▼────────────────────────────────────┐  │
│  │                    Evaluator                               │  │
│  │  • Tree-walking interpreter                                │  │
│  │  • Value representation                                    │  │
│  │  • Environment management                                  │  │
│  │  • Ownership tracking                                      │  │
│  └───────────────────────┬────────────────────────────────────┘  │
│                          │                                       │
│  ┌───────────────────────▼────────────────────────────────────┐  │
│  │               Compilation (Escape Hatch)                   │  │
│  │  • syn AST → TokenStream → rustc                           │  │
│  │  • Dynamic loading of compiled functions                   │  │
│  │  • Transparent hot-swap                                    │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

## Key Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `syn` AST complexity (hundreds of types) | Large implementation surface | Incremental coverage; "not yet implemented" for esoteric features |
| Ownership tracking performance overhead | Slower execution | Tiered checking; disable for hot paths |
| `rustc` compilation latency (1-5s) | Interrupts flow | Background compilation; never block the user |
| Macro system complexity | Extended timeline | Follow LFE's proven patterns; start simple |

---

## Future Vision: Fangorn

Beyond Phase 7, Treebeard can evolve into **Fangorn**, a BEAM-inspired runtime system:

- **v1.5:** Background compilation, perceived instant response
- **v2:** Cranelift JIT for hot functions (~10ms compile, 5x native speed)
- **v2.5:** Full in-memory operation, no filesystem dependencies
- **v3:** Lightweight processes, message passing, scheduling
- **v4:** Supervision trees, distribution, "let it crash" reliability

This evolution path is optional and will be evaluated after v1 stabilizes.

---

## Success Metrics

**Phase 1-4 (MVP):**
- Simple REPL eval: < 10ms latency
- Startup time: < 500ms
- Memory footprint: < 50MB idle

**Phase 5-7 (Complete):**
- Fibonacci(35) interpreted: < 2s
- Fibonacci(35) compiled: < 50ms  
- Hot reload: < 100ms
- Crate loading: functional for common crates

---

## Appendix: Current State (Oxur)

The project builds on existing Oxur infrastructure:

| Component | Status | Implication |
|-----------|--------|-------------|
| AST Bridge (S-exp ↔ syn) | 95% | Foundation ready |
| REPL Infrastructure | 60% | Session management exists |
| Core Evaluation | 25% | Needs proper evaluator |
| Macro System | 0% | Must build from scratch |

---

*"Don't be hasty."* — Treebeard
