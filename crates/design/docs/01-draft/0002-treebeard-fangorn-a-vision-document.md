---
number: 2
title: "Treebeard → Fangorn: A Vision Document"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-01-11
updated: 2026-01-11
state: Draft
supersedes: null
superseded-by: null
version: 2.0
---


# Treebeard → Fangorn: A Vision Document

**Date:** 2026-01-10
**Version:** 2.0
**Status:** Aspirational Roadmap
**Tagline:** *From interpreter to BEAM-like runtime, forged in Rust*

---

## Executive Summary

This document outlines a long-term vision for evolving Treebeard from a tree-walking interpreter into Fangorn, a BEAM-inspired runtime system built in Rust. The path is incremental: each phase delivers standalone value while laying foundations for the next.

The key insight is that **Rust can provide BEAM's operational characteristics** (process isolation, supervision, distribution, hot code loading) **with Rust's performance and safety guarantees**. This is not about replacing BEAM—it's about bringing BEAM's proven architectural patterns to the Rust ecosystem.

### What's New in v2.0

- **Tiered Execution Architecture** — Three-tier system (interpreter → Cranelift JIT → rustc native) for optimal performance/latency tradeoffs
- **In-Memory Compilation Analysis** — Detailed examination of paths to eliminate filesystem dependencies
- **Lunatic Assessment** — Evaluation of Lunatic as potential deployment target vs. building our own runtime
- **Refined Phase Structure** — v1.5 and v2.5 intermediate phases for smoother progression

---

## The Vision

```
"What if we built something with BEAM's operational characteristics
 but with Rust's performance and safety guarantees?"
```

BEAM's strengths are *architectural*, not implementation-specific:

- Isolated processes that can't corrupt each other
- Message passing as the only communication primitive
- "Let it crash" philosophy with supervisor recovery
- Hot code loading without downtime
- Distribution as a first-class concept

These ideas can be implemented in any language. Rust's ownership model actually *helps*—process isolation can be enforced at compile time, not just runtime.

---

## Terminology

Before diving in, let's establish precise terminology:

| Term | Definition | Applies To |
|------|------------|------------|
| **Interpreter** | Executes code by traversing AST directly | Treebeard v1-v2 |
| **VM (Virtual Machine)** | Executes bytecode via instruction pointer loop | NOT Treebeard (no bytecode) |
| **Runtime** | Execution environment + supporting infrastructure | Fangorn v3-v4 |
| **JIT** | Just-In-Time compilation to native code | Tier 1 (Cranelift) |
| **AOT** | Ahead-Of-Time compilation | Tier 2 (rustc) |

**Key distinction:** Treebeard is an **interpreter**, not a VM. It may never need bytecode—tiered execution with JIT and AOT compilation provides performance without a bytecode layer.

---

## Phase Overview

| Phase | Name | Term | Key Deliverable |
|-------|------|------|-----------------|
| **v1** | Treebeard | Tree-Walking Interpreter | Execute `syn` AST directly |
| **v1.5** | Treebeard | Tree-Walking Interpreter | Background compilation, perceived speed |
| **v2** | Treebeard | Interpreter + JIT | Cranelift JIT for hot functions |
| **v2.5** | Treebeard | Interpreter + JIT | Speculative compilation, in-memory modules |
| **v3** | Fangorn | Runtime System | Processes, scheduling, message passing |
| **v4** | Fangorn | Runtime System | Supervision, distribution, reliability |

---

## Tiered Execution Architecture

The core innovation in this vision is **tiered execution**—different execution strategies optimized for different use cases, with automatic promotion between tiers.

### The Three Tiers

```
┌─────────────────────────────────────────────────────────────────┐
│                    Treebeard Runtime                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Tier 0: Tree-Walking Interpreter (always available)            │
│  ├── Executes syn AST directly                                  │
│  ├── Ownership tracking built-in                                │
│  ├── ~50-100x slower than native                                │
│  ├── Instant startup, zero compilation delay                    │
│  └── Every function starts here                                 │
│                                                                 │
│  Tier 1: Cranelift JIT (for hot functions)                      │
│  ├── Triggered after N executions (configurable, default: 100)  │
│  ├── Compiles syn AST → Cranelift IR → machine code             │
│  ├── ~10ms compilation time per function                        │
│  ├── ~2-5x slower than LLVM-optimized                           │
│  ├── NO borrow checking (Tier 0 already validated)              │
│  └── Automatic promotion, no user action required               │
│                                                                 │
│  Tier 2: rustc + LLVM (for critical paths)                      │
│  ├── Triggered by user: (compile fib) or auto for very hot code │
│  ├── Full rustc compilation (background, non-blocking)          │
│  ├── ~1-5s compilation time                                     │
│  ├── Native performance (full LLVM optimization)                │
│  └── Full Rust semantics, production-quality code               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Tier Characteristics

| Tier | When Used | Compilation Time | Execution Speed | Use Case |
|------|-----------|------------------|-----------------|----------|
| **Tier 0** | First run, cold code | 0ms | ~100x slower | Exploration, debugging |
| **Tier 1** | Hot loops, frequently called | ~10ms | ~5x slower | Interactive development |
| **Tier 2** | Critical paths, production | ~2000ms | Native | Benchmarks, deployment |

### Why Three Tiers?

**The problem with two tiers:**

- Interpreter-only: Too slow for real work
- Interpreter + rustc: 2-second compilation pauses break flow

**The solution:** Cranelift as middle tier

- Fast enough to compile during a REPL pause (~10ms)
- Fast enough to run interactive code (~5x native)
- Bridges the gap between "instant but slow" and "slow to compile but fast"

### Automatic Tier Promotion

```
User types: (defn fib [n] ...)
            │
            ▼
┌─────────────────────────────────────┐
│ Tier 0: Interpret immediately       │
│ User sees result: 14930352          │
│ Execution count: 1                  │
└─────────────────────────────────────┘
            │
            │ (fib called 100+ times)
            ▼
┌─────────────────────────────────────┐
│ Background: Tier 1 JIT compiles     │
│ ~10ms, non-blocking                 │
│ Next call uses JIT version          │
└─────────────────────────────────────┘
            │
            │ (fib called 10,000+ times OR user requests)
            ▼
┌─────────────────────────────────────┐
│ Background: Tier 2 rustc compiles   │
│ ~2s, completely non-blocking        │
│ Swap in when ready                  │
└─────────────────────────────────────┘
```

**Key UX principle:** The user never waits. They always get *some* result immediately from whatever tier is available.

---

## The Path to In-Memory Execution

A major goal is eliminating filesystem dependencies. Here's the analysis of what's blocking us and how to overcome it.

### The Core Problem

`rustc` is an **ahead-of-time batch compiler**, not an incremental in-memory engine. It expects:

- Files on disk
- Complete compilation units (crates)
- To produce artifacts and exit

We want:

- Code in memory
- Incremental definitions
- Immediate execution
- Persistent sessions

### Three Paths Analyzed

#### Path 1: Make rustc Work In-Memory

**Verdict: Not practical**

- rustc is not a stable library (APIs change constantly)
- No "compile this string" API
- LLVM compilation is inherently slow (~100ms+ per function)
- Would require constant maintenance against rustc versions

**Who tried:** evcxr (Rust REPL) gave up and uses temp files + subprocess.

#### Path 2: Use Cranelift Instead of LLVM ✅ Recommended

**Verdict: Most promising path**

Cranelift is designed for JIT use cases:

- Fast compilation (~10ms per function)
- In-memory code generation
- No filesystem needed
- Production-ready (powers Wasmtime)

```
syn AST → Lower to Cranelift IR → Cranelift codegen → Machine code in memory
                                                              │
                                                    Call via function pointer
```

**Key insight:** We don't need rustc's borrow checker for JIT because Treebeard's Tier 0 already verified ownership at runtime. If code passed Tier 0, it's safe to JIT without re-checking.

#### Path 3: Compile to WASM, Execute via Wasmtime

**Verdict: Viable for deployment, not for REPL**

- WASM is ~1.5-2x slower than native
- Can't invoke rustc from inside WASM (no native escape hatch)
- Better for distribution than raw performance

**Use case:** Potentially useful for Fangorn v3-v4 as a deployment target, not for development REPL.

### The Recommended Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Treebeard Execution Engine                      │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    In-Memory Module Registry                    │   │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐             │   │
│  │  │ Module: core │ │ Module: user │ │ Module: repl │             │   │
│  │  │              │ │              │ │              │             │   │
│  │  │ fib: Tier 2  │ │ greet: Tier 0│ │ temp: Tier 0 │             │   │
│  │  │ map: Tier 1  │ │ calc: Tier 1 │ │              │             │   │
│  │  └──────────────┘ └──────────────┘ └──────────────┘             │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                    │                                   │
│                                    ▼                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Execution Dispatcher                       │   │
│  │                                                                 │   │
│  │   call(func_name, args) → match tier {                          │   │
│  │       Tier0 => interpreter.eval(ast, args),                     │   │
│  │       Tier1 => jit_code_ptr(args),                              │   │
│  │       Tier2 => native_code_ptr(args),                           │   │
│  │   }                                                             │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                        │
│  ┌───────────────────────┐  ┌───────────────────────┐                  │
│  │  Background Compiler  │  │    Tier Promoter      │                  │
│  │                       │  │                       │                  │
│  │  • Cranelift JIT      │  │  • Tracks call counts │                  │
│  │  • rustc subprocess   │  │  • Triggers promotion │                  │
│  │  • Non-blocking       │  │  • Swaps code safely  │                  │
│  └───────────────────────┘  └───────────────────────┘                  │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### What We Need to Build

#### For Tier 1 (Cranelift JIT)

New crate: `treebeard-jit`

```rust
pub struct JitCompiler {
    module: cranelift_jit::JITModule,
    ctx: cranelift::codegen::Context,
    func_ids: HashMap<String, FuncId>,
}

impl JitCompiler {
    /// Compile a syn function to native code
    pub fn compile(&mut self, func: &syn::ItemFn) -> Result<*const u8, JitError> {
        // 1. Lower syn AST to Cranelift IR
        let ir = self.lower_to_ir(func)?;

        // 2. Compile IR to machine code
        let func_id = self.module.declare_function(/* ... */)?;
        self.ctx.func = ir;
        self.module.define_function(func_id, &mut self.ctx)?;

        // 3. Finalize and get function pointer
        self.module.finalize_definitions()?;
        Ok(self.module.get_finalized_function(func_id))
    }

    fn lower_to_ir(&self, func: &syn::ItemFn) -> Result<Function, Error> {
        // The interesting part: syn::Expr → Cranelift instructions
        // This is substantial but well-defined work
    }
}
```

**Scope for Tier 1 JIT:**

- Numeric operations (arithmetic, comparisons)
- Control flow (if/else, loops, match on primitives)
- Function calls (to other JIT'd or native functions)
- Local variables

**Out of scope for Tier 1 (fall back to interpreter or Tier 2):**

- Complex pattern matching
- Trait method dispatch
- Anything requiring std library (println, Vec, etc.)

#### For Background Compilation (Tier 2)

```rust
pub struct BackgroundCompiler {
    pending: Arc<Mutex<VecDeque<CompileRequest>>>,
    completed: Arc<Mutex<HashMap<String, CompiledFunction>>>,
    worker: JoinHandle<()>,
}

impl BackgroundCompiler {
    pub fn request_compile(&self, name: String, ast: syn::ItemFn) {
        self.pending.lock().push_back(CompileRequest { name, ast });
        // Worker thread picks this up, compiles via rustc, loads .so
    }

    pub fn poll_completed(&self, name: &str) -> Option<CompiledFunction> {
        self.completed.lock().remove(name)
    }
}
```

---

## Phase Details

### Phase 1: Treebeard v1 — The Interpreter

**Status:** In Progress
**Timeline:** ~16-20 weeks
**Term:** Tree-Walking Interpreter

#### What It Is

A tree-walking interpreter that directly executes Rust's `syn` AST. This is Tier 0—the foundation everything else builds on.

#### Key Features

- **Direct AST execution** — No bytecode compilation step
- **Instant startup** — Parse and run immediately
- **REPL integration** — Interactive development workflow
- **Ownership tracking** — Runtime verification of move/borrow semantics
- **Compilation escape hatch** — Compile hot functions to native code via `rustc` (file-based initially)
- **`LanguageFrontend` trait** — Clean abstraction for multiple language frontends

#### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Language Frontend                       │
│  (Oxur, or any language producing syn AST)                  │
│                                                             │
│  Implements: LanguageFrontend trait                         │
│    - parse(source) → Vec<syn::Item>                         │
│    - expand_macros(items) → Vec<syn::Item>                  │
│    - format_error(error) → String                           │
└─────────────────────────┬───────────────────────────────────┘
                          │ syn AST
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                     Treebeard Core                          │
│                                                             │
│  • Evaluator        — Interprets syn AST (Tier 0)           │
│  • Environment      — Variable bindings (flat scope)        │
│  • Value            — Runtime value representation          │
│  • OwnershipTracker — Runtime ownership verification        │
│                                                             │
│  Escape Hatch: syn AST → rustc → native code (Tier 2)       │
└─────────────────────────────────────────────────────────────┘
```

#### Deliverables

- [ ] Core evaluator for `syn::Expr` and `syn::Stmt`
- [ ] Environment with flat scope model
- [ ] Value representation (inline + heap + native)
- [ ] Ownership tracking (8 bytes per value)
- [ ] Closure capture with upvalues
- [ ] REPL integration with Oxur
- [ ] Compilation escape hatch via `rustc` (file-based)
- [ ] Crate loading for Rust ecosystem access

#### Success Criteria

```
oxur> (defn fib [n]
        (if (<= n 1)
            n
            (+ (fib (- n 1)) (fib (- n 2)))))
oxur> (time (fib 35))
Elapsed: 1500ms
14930352
oxur> (compile fib)
✓ Compiled fib to native code
oxur> (time (fib 35))
Elapsed: 15ms
14930352
```

---

### Phase 1.5: Treebeard v1.5 — Background Compilation

**Status:** Future
**Timeline:** ~4 weeks after v1
**Term:** Tree-Walking Interpreter (with background compilation)

#### What It Is

Enhancement to v1 that makes compilation non-blocking. The REPL never pauses waiting for rustc.

#### Key Features

- **Background rustc compilation** — Compile in separate thread
- **Speculative compilation** — Start compiling before user explicitly requests
- **Compilation cache** — Don't recompile unchanged functions
- **Progress indication** — Show compilation status without blocking

#### Architecture Addition

```
┌─────────────────────────────────────────────────────────────┐
│                  Background Compiler (new)                  │
│                                                             │
│  • Worker thread pool for rustc invocations                 │
│  • Compilation queue with priority                          │
│  • Result cache (function → compiled code)                  │
│  • Hot-swap mechanism for completed compilations            │
└─────────────────────────────────────────────────────────────┘
```

#### Success Criteria

```
oxur> (defn fib [n] ...)        ; Returns immediately
oxur> (fib 35)                   ; Interpreted, 1500ms
oxur> ; User keeps typing...
oxur> ; [background: compiling fib...]
oxur> (fib 35)                   ; Still interpreted, but...
oxur> ; [background: fib compiled ✓]
oxur> (fib 35)                   ; Native! 15ms
```

---

### Phase 2: Treebeard v2 — Cranelift JIT

**Status:** Future
**Timeline:** ~8-12 weeks after v1.5
**Term:** Interpreter + JIT

#### What It Is

Introduction of Tier 1: Cranelift-based JIT compilation for hot functions. This is the key to "in-memory feel" without filesystem dependencies.

#### Key Features

- **Cranelift JIT compiler** — Fast in-memory compilation (~10ms)
- **Automatic tier promotion** — Hot functions auto-promote from Tier 0 to Tier 1
- **No borrow checking in JIT** — Tier 0 already validated ownership
- **Execution counting** — Track function call frequency
- **In-memory code storage** — No temp files for Tier 1

#### Architecture

```
┌───────────────────────────────────────────────────────────┐
│                  Treebeard v2 Architecture                │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐  │
│  │   Tier 0    │     │   Tier 1    │     │   Tier 2    │  │
│  │ Interpreter │ ──► │ Cranelift   │ ──► │   rustc     │  │
│  │             │     │    JIT      │     │   + LLVM    │  │
│  └─────────────┘     └─────────────┘     └─────────────┘  │
│        │                   │                   │          │
│        │ 100 calls         │ 10K calls         │          │
│        └───────────────────┴───────────────────┘          │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐  │
│  │              Tier Promotion Manager                 │  │
│  │  • Call counting per function                       │  │
│  │  • Promotion threshold configuration                │  │
│  │  • Safe hot-swap during execution                   │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐  │
│  │              Cranelift JIT Engine                   │  │
│  │  • syn::Expr → Cranelift IR lowering                │  │
│  │  • Function signature handling                      │  │
│  │  • Memory management for JIT code                   │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

#### New Crate: `treebeard-jit`

```rust
// Core JIT compiler interface
pub trait JitCompilable {
    fn lower_to_ir(&self, ctx: &mut JitContext) -> Result<(), JitError>;
}

impl JitCompilable for syn::ExprBinary {
    fn lower_to_ir(&self, ctx: &mut JitContext) -> Result<(), JitError> {
        self.left.lower_to_ir(ctx)?;
        self.right.lower_to_ir(ctx)?;
        match self.op {
            BinOp::Add(_) => ctx.emit_add(),
            BinOp::Sub(_) => ctx.emit_sub(),
            // ...
        }
    }
}
```

#### Deliverables

- [ ] `treebeard-jit` crate with Cranelift integration
- [ ] `syn::Expr` → Cranelift IR lowering for numeric types
- [ ] `syn::Expr` → Cranelift IR lowering for control flow
- [ ] Execution counter per function
- [ ] Tier promotion manager
- [ ] Safe code hot-swapping

#### Success Criteria

```
oxur> (defn fib [n] ...)
oxur> (time (loop [i 0] (when (< i 200) (fib 20) (recur (+ i 1)))))
; First ~100 iterations: interpreted (~50ms each)
; Remaining iterations: JIT compiled (~5ms each)
; Total time: much less than 200 * 50ms
Elapsed: 1200ms   ; vs 10000ms if all interpreted
```

---

### Phase 2.5: Treebeard v2.5 — In-Memory Everything

**Status:** Future
**Timeline:** ~4 weeks after v2
**Term:** Interpreter + JIT (fully in-memory)

#### What It Is

Complete elimination of filesystem dependencies for normal operation. Modules live in memory, hot loading works seamlessly.

#### Key Features

- **In-memory module registry** — No file juggling
- **Hot code swapping** — Redefine functions seamlessly
- **Module versioning** — Track which version is running
- **Speculative compilation** — Predict and pre-compile likely-to-be-called functions

#### Deliverables

- [ ] `ModuleRegistry` with in-memory storage
- [ ] Function versioning and safe replacement
- [ ] Speculative compilation based on call graph analysis
- [ ] Remove filesystem dependency from Tier 0 and Tier 1 entirely

#### Success Criteria

```
oxur> (defn greet [name] (str "Hello, " name))
oxur> (greet "World")
"Hello, World"
oxur> (defn greet [name] (str "Greetings, " name "!"))
✓ Hot-loaded: greet (v1 → v2)
oxur> (greet "World")
"Greetings, World!"
; No temp files created, no filesystem access
```

---

### Phase 3: Fangorn v3 — The Runtime

**Status:** Future
**Timeline:** TBD (after v2.5 stabilizes)
**Term:** Runtime System

#### What It Is

Introduction of BEAM-style lightweight processes, message passing, and scheduling. This is where Treebeard evolves into Fangorn—a true runtime system.

#### Key Features

- **Lightweight processes** — Not OS threads; thousands can run concurrently
- **Process isolation** — One process cannot corrupt another's memory
- **Message passing** — The only way processes communicate
- **Mailboxes** — Each process has an inbox for messages
- **Process registry** — Named processes (like Erlang's `whereis`)
- **Scheduler** — Preemptive, reduction-counted (like BEAM)
- **ETS-like storage** — Shared in-memory tables

#### Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        Fangorn Node                          │
│                                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐         │
│  │Process 1 │ │Process 2 │ │Process 3 │ │Process 4 │         │
│  │  (Oxur)  │ │  (Oxur)  │ │  (Rust)  │ │  (Oxur)  │         │
│  │          │ │          │ │          │ │          │         │
│  │ [mailbox]│ │ [mailbox]│ │ [mailbox]│ │ [mailbox]│         │
│  │ [heap]   │ │ [heap]   │ │ [heap]   │ │ [heap]   │         │
│  │ [tiers]  │ │ [tiers]  │ │ [native] │ │ [tiers]  │         │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘         │
│       │            │            │            │               │
│       └────────────┴─────┬──────┴────────────┘               │
│                          │                                   │
│  ┌───────────────────────┴────────────────────────────────┐  │
│  │                     Scheduler                          │  │
│  │  • Preemptive (reduction counting)                     │  │
│  │  • Fair scheduling across processes                    │  │
│  │  • Work stealing (optional)                            │  │
│  │  • Tier-aware (JIT processes get more reductions)      │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Shared Storage (ETS-like)                 │  │
│  │  • Named tables                                        │  │
│  │  • Concurrent read, serialized write                   │  │
│  │  • Process-owned or public                             │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

#### Why Rust Helps Here

Rust's ownership model provides guarantees that BEAM enforces at runtime:

```rust
// Message passing moves ownership — enforced at compile time!
fn send(target: ProcessId, msg: Message) {
    // `msg` is moved, not copied (unless Clone)
    // No shared mutable state possible
    // This IS Erlang's semantics, verified by rustc
}

// Process heaps are isolated by the type system
struct Process {
    mailbox: Receiver<Message>,
    heap: ProcessHeap,  // Other processes can't access this
    tiers: TierState,   // Per-process JIT state
}
```

#### Deliverables

- [ ] `Process` struct with isolated heap and mailbox
- [ ] `Scheduler` with preemptive scheduling
- [ ] `send` / `receive` primitives
- [ ] Process spawning and linking
- [ ] Process registry (named processes)
- [ ] `Table` (ETS-like shared storage)
- [ ] Process monitoring (detect crashes)

#### Success Criteria

```oxur
oxur> (defn counter [n]
        (receive
          [:inc] (counter (+ n 1))
          [:get from] (do (send from n)
                          (counter n))))

oxur> (def c (spawn counter 0))
#PID<0.42.0>

oxur> (send c [:inc])
oxur> (send c [:inc])
oxur> (send c [:inc])
oxur> (send c [:get (self)])
oxur> (receive [n] n)
3
```

---

### Phase 4: Fangorn v4 — Reliability & Distribution

**Status:** Future
**Timeline:** TBD (after v3 stabilizes)
**Term:** Runtime System

#### What It Is

The full BEAM-inspired reliability story: supervision trees, node distribution, and the operational characteristics that let BEAM power telephone switches.

#### Key Features

- **Supervisors** — Restart failed processes automatically
- **Restart strategies** — one_for_one, one_for_all, rest_for_one
- **Supervision trees** — Hierarchical fault isolation
- **Node distribution** — Multiple Fangorn nodes talking to each other
- **Location transparency** — Send messages to processes on other nodes
- **Hot code loading** — Update code without stopping the system
- **DETS-like persistence** — Durable storage across restarts

#### Architecture

```
┌────────────────────────────────────────────────────────┐
│                     Fangorn Cluster                    │
│                                                        │
│  ┌─────────────────────┐      ┌─────────────────────┐  │
│  │      Node A         │      │      Node B         │  │
│  │                     │      │                     │  │
│  │  ┌───────────────┐  │      │  ┌───────────────┐  │  │
│  │  │  Supervisor   │  │      │  │  Supervisor   │  │  │
│  │  │   ┌─────────┐ │  │      │  │   ┌─────────┐ │  │  │
│  │  │   │Worker 1 │ │  │◄────►│  │   │Worker 3 │ │  │  │
│  │  │   │Worker 2 │ │  │      │  │   │Worker 4 │ │  │  │
│  │  │   └─────────┘ │  │      │  │   └─────────┘ │  │  │
│  │  └───────────────┘  │      │  └───────────────┘  │  │
│  │                     │      │                     │  │
│  │  [ETS Tables]       │      │  [ETS Tables]       │  │
│  │  [DETS Storage]     │      │  [DETS Storage]     │  │
│  └─────────────────────┘      └─────────────────────┘  │
│                                                        │
│              Distribution Protocol (QUIC)              │
└────────────────────────────────────────────────────────┘
```

#### Deliverables

- [ ] `Supervisor` with restart strategies
- [ ] Supervision tree construction and management
- [ ] `Node` with distribution protocol
- [ ] Cross-node message passing
- [ ] Location-transparent process references
- [ ] Hot code loading at runtime level
- [ ] `DurableTable` (DETS-like persistence)
- [ ] Cluster membership and discovery

#### Success Criteria

```oxur
;; Define a supervision tree
(def tree
  (supervisor :one-for-one
    [(worker :counter counter-init)
     (worker :logger logger-init)
     (supervisor :one-for-all
       [(worker :db-writer db-init)
        (worker :db-reader db-init)])]))

;; Start the tree
(def sup (start-supervisor tree))

;; Kill a worker — supervisor restarts it automatically
(exit (whereis :counter) :kill)
;; Counter is back!

;; Connect to another node
(connect-node "fangorn@192.168.1.2")

;; Send message to process on remote node
(send {remote-node :logger} [:log "Hello from Node A"])
```

---

## Lunatic: Build vs. Use Analysis

[Lunatic](https://lunatic.solutions/) is a BEAM-inspired runtime for WASM. We evaluated it as a potential shortcut.

### What Lunatic Offers

| Feature | Lunatic | Build Our Own |
|---------|---------|---------------|
| Process isolation | ✅ Via WASM instances | Must build |
| Message passing | ✅ Built-in | Must build |
| Supervision | ✅ Built-in | Must build |
| Distribution | ✅ Built-in (QUIC) | Must build |
| Performance | ~1.5-2x slower (WASM) | Native speed |
| Native compilation | ❌ Can't call rustc from WASM | ✅ Full rustc access |

### Verdict

**Lunatic doesn't replace Treebeard—it would host it.**

- You still need to build the interpreter (Treebeard)
- Lunatic provides the process layer, but at a performance cost
- Can't use rustc escape hatch from inside WASM

**Recommendation:**

- **v1-v2:** Build natively (need rustc escape hatch, need performance)
- **v3-v4:** Evaluate Lunatic as deployment target for distributed systems
- **Development REPL:** Always native (need speed, need compilation)

### Hybrid Path (Future Option)

```
Development (native):
  Oxur → Treebeard (native) → Fast REPL with all tiers

Production (Lunatic):
  Oxur → Compile to WASM → Deploy on Lunatic → Distribution "for free"
```

This preserves fast development while getting Lunatic's operational benefits for deployment.

---

## The Erlang Parallel

For reference, mapping Erlang/OTP concepts to the Oxur ecosystem:

| Erlang/OTP | Oxur Ecosystem | Description |
|------------|----------------|-------------|
| Erlang | Oxur | The language |
| BEAM | (none — tiered execution) | We use interpreter + JIT + AOT instead of bytecode VM |
| OTP | Fangorn | Runtime system (processes, supervisors, distribution) |
| ERTS | Fangorn | Full runtime (scheduler, memory, ports, distribution) |
| ETS | `Table` | In-memory shared storage |
| DETS | `DurableTable` | Persistent storage |
| `gen_server` | TBD | Generic server behavior |
| `supervisor` | `Supervisor` | Process supervision |

**Key difference:** BEAM needs bytecode because Erlang can't easily compile to native code. We have tiered execution: interpreter (instant) → Cranelift JIT (fast compile, good perf) → rustc (slow compile, best perf).

---

## Design Principles

### 1. Incremental Value

Each phase delivers standalone value:

- **v1:** Usable interpreter with REPL
- **v1.5:** Non-blocking compilation, better UX
- **v2:** JIT compilation, "instant" feel
- **v2.5:** Full in-memory operation
- **v3:** Concurrency for real applications
- **v4:** Production reliability

No phase depends on promises of future phases.

### 2. Tiered Execution, Not Bytecode

Traditional VMs compile to bytecode. We take a different approach:

- **Tier 0:** Interpret directly (instant, slow execution)
- **Tier 1:** JIT via Cranelift (fast compile, good execution)
- **Tier 2:** AOT via rustc (slow compile, best execution)

This gives us the benefits of bytecode (portability, hot loading) without the complexity.

### 3. Rust's Strengths, Not Fighting Them

Rust's ownership model is an *asset*, not an obstacle:

- Process isolation enforced at compile time
- Zero-copy message passing where safe
- No GC pauses (unlike BEAM)
- Native performance for hot paths

### 4. The Thin Layer Principle

Like LFE delegates to BEAM, Fangorn delegates to Rust:

- Type checking → `rustc` (at Tier 2)
- Optimization → LLVM (at Tier 2), Cranelift (at Tier 1)
- Memory safety → Rust's borrow checker + Treebeard's ownership tracking
- Native performance → Tier 1 and Tier 2 compilation

### 5. Never Block the User

The REPL must always feel responsive:

- First execution: Interpret immediately (Tier 0)
- Hot functions: Auto-promote in background
- Compilation: Always non-blocking
- User never waits for rustc

---

## Prior Art & Inspiration

| Project | Relationship to Fangorn |
|---------|------------------------|
| **BEAM/OTP** | Primary inspiration for process model and supervision |
| **LFE** | Inspiration for "thin layer" principle |
| **Lunatic** | Potential deployment target for v3-v4; source of patterns |
| **Bastion** | Rust actor framework with supervision |
| **Tokio** | Foundation for async I/O (potential building block) |
| **Rhai** | Patterns for tree-walking interpretation |
| **Miri** | Patterns for ownership tracking |
| **Cranelift** | JIT compilation engine for Tier 1 |
| **V8/SpiderMonkey** | Inspiration for tiered JIT architecture |

---

## Open Questions

### Resolved by v1 Development

- How expensive is ownership tracking in practice?
- What's the optimal `Value` representation size?
- How much of `syn` AST do we actually need?

### Resolved by v2 Development

- What subset of `syn` can Cranelift JIT handle?
- Optimal tier promotion thresholds?
- How to handle JIT failures gracefully (fall back to interpreter)?

### Resolved by v2.5 Development

- What's the right granularity for incremental compilation?
- How to handle circular dependencies in hot loading?
- Speculative compilation: which functions to pre-compile?

### Resolved by v3 Development

- Reduction counting: what counts as a reduction?
- Scheduler: work-stealing vs. shared queue?
- Process heap: separate allocator or standard?
- Message passing: copy always, or move when possible?

### Resolved by v4 Development

- Distribution protocol: custom or QUIC?
- Hot code loading: how to handle state migration?
- Cluster membership: static config or dynamic discovery?
- Should we target Lunatic for distribution instead of building our own?

---

## Timeline Speculation

**Caveat:** These are rough estimates. Each phase should be re-evaluated after the previous phase stabilizes.

| Phase | Effort | Cumulative | Key Risk |
|-------|--------|------------|----------|
| v1 | 16-20 weeks | 16-20 weeks | syn AST complexity |
| v1.5 | 4 weeks | 20-24 weeks | Low risk |
| v2 | 8-12 weeks | 28-36 weeks | Cranelift IR lowering |
| v2.5 | 4 weeks | 32-40 weeks | Hot-swap correctness |
| v3 | 16-24 weeks | 48-64 weeks | Scheduler correctness |
| v4 | 16-32 weeks | 64-96 weeks | Distribution protocol |

**Total to "telephone switch reliability":** 16-24 months from v1 completion.

---

## Conclusion

The path from Treebeard to Fangorn is clear:

1. **Interpreter** — Execute code immediately (v1)
2. **Background compilation** — Never block (v1.5)
3. **JIT** — Fast in-memory compilation (v2)
4. **In-memory everything** — No filesystem (v2.5)
5. **Processes** — Concurrency (v3)
6. **Supervision + Distribution** — Reliability (v4)

Each step builds on the previous. The tiered execution architecture ensures users never wait, while still achieving native performance for hot code.

The terminology evolves naturally:

- **v1-v2.5:** "Treebeard" — a tree-walking interpreter with JIT
- **v3-v4:** "Fangorn" — a runtime system

No bytecode required. No "VM" label needed. Just a steady evolution from interpreter to runtime, with BEAM's battle-tested patterns implemented in Rust.

---

*"Don't be hasty."*

— Treebeard, on the value of tree-walking interpretation

*"The trees have grown wild and dangerous. But there is still hope."*

— Future Fangorn documentation, probably

---

**End of Vision Document**
