# Treebeard Planning Session Bootstrap

**Purpose:** Quick-start guide for bootstrapping new Claude sessions to generate implementation stage documents for Claude Code.

---

## Bootstrap Prompt

Copy and paste this into a new Claude session:

```
I'm working on Treebeard, a tree-walking interpreter for Rust's `syn` AST. I need help creating detailed implementation stage documents for Claude Code.

I've uploaded:
1. The main implementation guide (0001-treebeard-implementation-guide-v3.md) - architecture and design decisions
2. The implementation stages doc (treebeard-implementation-stages.md) - breakdown of phases into stages
3. The CLAUDE.md for the project - conventions and patterns

I need you to create a detailed implementation document for Stage [X.Y: Name].

The format should match the previous stage docs:
- Objective and overview
- File structure showing where new code goes
- Complete type definitions with full code
- All method implementations
- Error types
- Comprehensive test cases (15-25 tests)
- Completion checklist
- Design notes explaining key decisions
- "Next Stage" pointer

The doc should be self-contained so Claude Code can implement the entire stage without referencing other documents.
```

---

## Files to Upload

### Always Required

| File | Purpose |
|------|---------|
| `0001-treebeard-implementation-guide-v3.md` | Architecture, design decisions, detailed specs |
| `treebeard-implementation-stages.md` | Stage breakdown with descriptions |
| `treebeard-CLAUDE.md` | Project conventions and patterns |

### Optional (Helpful Context)

| File | When to Include |
|------|-----------------|
| `treebeard-project-plan.md` | For high-level phase context |
| Previous stage doc (e.g., `stage-1.2-environment.md`) | For format/style consistency |
| `0002-treebeard-fangorn-a-vision-document.md` | For later phases where future direction matters |

---

## Stage Reference

### Phase 1: Core Evaluator (6 stages)

| Stage | Name | Description |
|-------|------|-------------|
| 1.1 | Value Representation | `Value` enum covering Rust's runtime types |
| 1.2 | Environment | Scoped bindings with `Environment` struct |
| 1.3 | Basic Expressions | Literals, paths, binary ops, unary ops |
| 1.4 | Control Flow | `if`/`else`, `match`, `loop`/`while`/`for`, `break`/`continue` |
| 1.5 | Functions | `fn` definitions, calls, argument passing, `return` |
| 1.6 | Statements & Blocks | `let` bindings, expression statements, block scoping |

### Phase 2: Frontend Trait (3 stages)

| Stage | Name | Description |
|-------|------|-------------|
| 2.1 | Trait Definition | `LanguageFrontend` trait with parse, expand, format methods |
| 2.2 | Rust Frontend | Frontend parsing Rust source via `syn` |
| 2.3 | Oxur Frontend | Integrate Oxur's AST Bridge as frontend |

### Phase 3: Macro System (5 stages)

| Stage | Name | Description |
|-------|------|-------------|
| 3.1 | Macro Environment | `MacroEnvironment` separate from runtime |
| 3.2 | Quasiquote | Quasiquote/unquote/unquote-splicing for templates |
| 3.3 | Defmacro | `defmacro` form registering syntax transformers |
| 3.4 | Expansion Pass | Macro expansion pass before evaluation |
| 3.5 | Hygiene | Gensym and hygiene to prevent variable capture |

### Phase 4: REPL Integration (4 stages)

| Stage | Name | Description |
|-------|------|-------------|
| 4.1 | Evaluation Loop | Connect REPL to evaluator, multi-line input |
| 4.2 | Commands | `:help`, `:type`, `:env`, `:load`, `:quit`, etc. |
| 4.3 | Completion | Tab completion for bindings and keywords |
| 4.4 | Output & History | Pretty-print values, persist history |

### Phase 5: Closures & Ownership (4 stages)

| Stage | Name | Description |
|-------|------|-------------|
| 5.1 | Closure Values | Extend `Value` for closures, capture environment |
| 5.2 | Upvalue Handling | By-value and by-reference capture, nested closures |
| 5.3 | Ownership State | Ownership tracking (Owned/Borrowed/Moved) |
| 5.4 | Ownership Checks | Enforce ownership rules, error messages |

### Phase 6: Compilation (4 stages)

| Stage | Name | Description |
|-------|------|-------------|
| 6.1 | Code Generation | `syn` AST → TokenStream → Rust source |
| 6.2 | Background Compilation | Invoke `rustc` in background, produce `cdylib` |
| 6.3 | Dynamic Loading | Load compiled functions via `libloading` |
| 6.4 | Hot Swap | Replace interpreted with compiled, caching |

### Phase 7: Crate Loading (4 stages)

| Stage | Name | Description |
|-------|------|-------------|
| 7.1 | Cargo Integration | Fetch and build external crates |
| 7.2 | Symbol Extraction | Extract function symbols, build registry |
| 7.3 | Type Bridging | Convert between `Value` and Rust types |
| 7.4 | Require Command | `(require "crate")` command integration |

---

## Example Prompt for Stage 1.3

```
I'm working on Treebeard, a tree-walking interpreter for Rust's `syn` AST. I need help creating detailed implementation stage documents for Claude Code.

I've uploaded:
1. The main implementation guide (0001-treebeard-implementation-guide-v3.md)
2. The implementation stages doc (treebeard-implementation-stages.md)
3. The CLAUDE.md for the project
4. The previous stage doc (treebeard-stage-1.2-environment.md) for format reference

I need you to create a detailed implementation document for Stage 1.3: Basic Expressions.

This stage covers evaluating literals, paths (variable references), binary operations, and unary operations.

The format should match stage-1.2: objective, file structure, complete code, tests, checklist, design notes, next stage pointer.
```

---

## Stage Document Checklist

A good stage document includes:

- [ ] **Header**: Stage number, phase, prerequisites, estimated effort
- [ ] **Objective**: One paragraph on what we're building
- [ ] **Overview**: Explanation of the approach/design
- [ ] **File Structure**: Where new files go in the crate
- [ ] **Complete Type Definitions**: Full code, not pseudocode
- [ ] **All Method Implementations**: Ready to copy-paste
- [ ] **Error Types**: With `thiserror` derives
- [ ] **Module Exports**: Updates to `mod.rs` and `lib.rs`
- [ ] **Test Cases**: 15-25 tests covering happy path, errors, edge cases
- [ ] **Completion Checklist**: Checkboxes for all deliverables
- [ ] **Design Notes**: Explain non-obvious decisions
- [ ] **Next Stage**: Pointer to what comes after

---

## Notes

- Each stage document should be **self-contained** — Claude Code shouldn't need to reference other docs
- Include **complete, working code** — not snippets or pseudocode
- Tests should be **comprehensive** — Claude Code uses them to verify correctness
- The **completion checklist** helps Claude Code know when it's done

---

**Last Updated:** 2026-01-11
