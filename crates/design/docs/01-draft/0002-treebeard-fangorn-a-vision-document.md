---
number: 2
title: "Treebeard → Fangorn: A Vision Document"
author: "the type"
component: All
tags: [change-me]
created: 2026-01-11
updated: 2026-01-11
state: Draft
supersedes: null
superseded-by: null
version: 1.0
---

# Treebeard → Fangorn: A Vision Document

**Date:** 2026-01-10  
**Status:** Aspirational Roadmap  
**Tagline:** *From interpreter to BEAM-like runtime, forged in Rust*

---

## Executive Summary

This document outlines a long-term vision for evolving Treebeard from a tree-walking interpreter into Fangorn, a BEAM-inspired runtime system built in Rust. The path is incremental: each phase delivers standalone value while laying foundations for the next.

The key insight is that **Rust can provide BEAM's operational characteristics** (process isolation, supervision, distribution, hot code loading) **with Rust's performance and safety guarantees**. This is not about replacing BEAM—it's about bringing BEAM's proven architectural patterns to the Rust ecosystem.

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

## Phase Overview

| Phase | Name | Term | Key Deliverable |
|-------|------|------|-----------------|
| **v1** | Treebeard | Tree-Walking Interpreter | Execute `syn` AST directly |
| **v2** | Treebeard | Tree-Walking Interpreter | In-memory compilation, hot loading |
| **v3** | Fangorn | Runtime System | Processes, scheduling, message passing |
| **v4** | Fangorn | Runtime System | Supervision, distribution, "telephone switch" reliability |

**Note:** "VM" would only apply if a bytecode tier is added. The current vision achieves full functionality via tree-walking + native compilation escape hatch, potentially never requiring bytecode.

---

## Phase 1: Treebeard v1 — The Interpreter

**Status:** In Progress  
**Timeline:** ~16-20 weeks  
**Term:** Tree-Walking Interpreter

### What It Is

A tree-walking interpreter that directly executes Rust's `syn` AST. Any language that compiles to `syn` AST (Oxur, or others) can use Treebeard for immediate execution.

### Key Features

- **Direct AST execution** — No bytecode compilation step
- **Instant startup** — Parse and run immediately
- **REPL integration** — Interactive development workflow
- **Ownership tracking** — Runtime verification of move/borrow semantics
- **Compilation escape hatch** — Compile hot functions to native code via `rustc`
- **`LanguageFrontend` trait** — Clean abstraction for multiple language frontends

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Language Frontend                        │
│  (Oxur, or any language producing syn AST)                  │
│                                                              │
│  Implements: LanguageFrontend trait                          │
│    - parse(source) → Vec<syn::Item>                          │
│    - expand_macros(items) → Vec<syn::Item>                   │
│    - format_error(error) → String                            │
└─────────────────────────┬───────────────────────────────────┘
                          │ syn AST
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                     Treebeard Core                           │
│                                                              │
│  • Evaluator        — Interprets syn AST                     │
│  • Environment      — Variable bindings (flat scope)         │
│  • Value            — Runtime value representation           │
│  • OwnershipTracker — Runtime ownership verification         │
│                                                              │
│  Escape Hatch: syn AST → rustc → native code                 │
└─────────────────────────────────────────────────────────────┘
```

### Deliverables

- [ ] Core evaluator for `syn::Expr` and `syn::Stmt`
- [ ] Environment with flat scope model
- [ ] Value representation (inline + heap + native)
- [ ] Ownership tracking (8 bytes per value)
- [ ] Closure capture with upvalues
- [ ] REPL integration with Oxur
- [ ] Compilation escape hatch via `rustc`
- [ ] Crate loading for Rust ecosystem access

### Success Criteria

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

## Phase 2: Treebeard v2 — In-Memory Infrastructure

**Status:** Future  
**Timeline:** TBD (after v1 stabilizes)  
**Term:** Tree-Walking Interpreter (still)

### What It Is

Evolution of Treebeard to eliminate filesystem-based compilation workflow, replacing it with in-memory module management and compilation.

### Key Features

- **In-memory module registry** — No more file juggling
- **Incremental compilation** — Only recompile changed definitions
- **Background compilation** — Don't block REPL while compiling
- **Hot code swapping** — Replace function definitions without restart
- **Module versioning** — Track which version of a function is running

### Architecture Evolution

```
v1 (File-Based):
  Source → AST → Write .rs file → rustc → Load .so → Execute
                      ↑
                 Filesystem overhead

v2 (In-Memory):
  Source → AST → In-memory compile → Hot load → Execute
                      ↑
                 No filesystem!
```

### Why This Matters

The filesystem-based workflow in v1 is a **bootstrap constraint**, not a design goal. Removing it:

1. **Faster iteration** — No file I/O latency
2. **Cleaner semantics** — Modules exist in memory, not scattered files
3. **Foundation for processes** — Each process can have its own module table
4. **Enables hot loading** — Swap code without restarting

### Deliverables

- [ ] `ModuleRegistry` — In-memory storage of compiled modules
- [ ] `IncrementalCompiler` — Track dependencies, recompile minimally
- [ ] `BackgroundCompiler` — Compile without blocking REPL
- [ ] `HotLoader` — Swap function implementations at runtime
- [ ] Remove filesystem dependency from core workflow

### Success Criteria

```
oxur> (defn greet [name] (str "Hello, " name))
oxur> (greet "World")
"Hello, World"
oxur> (defn greet [name] (str "Greetings, " name "!"))  ; Redefine
✓ Hot-loaded: greet
oxur> (greet "World")
"Greetings, World!"
```

---

## Phase 3: Fangorn v3 — The Runtime

**Status:** Future  
**Timeline:** TBD (after v2 stabilizes)  
**Term:** Runtime System

### What It Is

Introduction of BEAM-style lightweight processes, message passing, and scheduling. This is where Treebeard evolves into Fangorn—a true runtime system.

### Key Features

- **Lightweight processes** — Not OS threads; thousands can run concurrently
- **Process isolation** — One process cannot corrupt another's memory
- **Message passing** — The only way processes communicate
- **Mailboxes** — Each process has an inbox for messages
- **Process registry** — Named processes (like Erlang's `whereis`)
- **Scheduler** — Preemptive, reduction-counted (like BEAM)
- **ETS-like storage** — Shared in-memory tables

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Fangorn Node                          │
│                                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │Process 1 │ │Process 2 │ │Process 3 │ │Process 4 │       │
│  │  (Oxur)  │ │  (Oxur)  │ │  (Rust)  │ │  (Oxur)  │       │
│  │          │ │          │ │          │ │          │       │
│  │ [mailbox]│ │ [mailbox]│ │ [mailbox]│ │ [mailbox]│       │
│  │ [heap]   │ │ [heap]   │ │ [heap]   │ │ [heap]   │       │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘       │
│       │            │            │            │              │
│       └────────────┴─────┬──────┴────────────┘              │
│                          │                                   │
│  ┌───────────────────────┴────────────────────────────────┐ │
│  │                     Scheduler                           │ │
│  │  • Preemptive (reduction counting)                      │ │
│  │  • Fair scheduling across processes                     │ │
│  │  • Work stealing (optional)                             │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Shared Storage (ETS-like)                   │ │
│  │  • Named tables                                          │ │
│  │  • Concurrent read, serialized write                     │ │
│  │  • Process-owned or public                               │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Why Rust Helps Here

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
}
```

BEAM copies messages between process heaps. Rust can do the same, OR use ownership transfer for zero-copy messaging when safe—a potential performance win.

### Deliverables

- [ ] `Process` struct with isolated heap and mailbox
- [ ] `Scheduler` with preemptive scheduling
- [ ] `send` / `receive` primitives
- [ ] Process spawning and linking
- [ ] Process registry (named processes)
- [ ] `Table` (ETS-like shared storage)
- [ ] Process monitoring (detect crashes)

### Success Criteria

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

## Phase 4: Fangorn v4 — Reliability & Distribution

**Status:** Future  
**Timeline:** TBD (after v3 stabilizes)  
**Term:** Runtime System

### What It Is

The full BEAM-inspired reliability story: supervision trees, node distribution, and the operational characteristics that let BEAM power telephone switches.

### Key Features

- **Supervisors** — Restart failed processes automatically
- **Restart strategies** — one_for_one, one_for_all, rest_for_one
- **Supervision trees** — Hierarchical fault isolation
- **Node distribution** — Multiple Fangorn nodes talking to each other
- **Location transparency** — Send messages to processes on other nodes
- **Hot code loading** — Update code without stopping the system
- **DETS-like persistence** — Durable storage across restarts

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Fangorn Cluster                           │
│                                                              │
│  ┌─────────────────────┐      ┌─────────────────────┐       │
│  │      Node A         │      │      Node B         │       │
│  │                     │      │                     │       │
│  │  ┌───────────────┐  │      │  ┌───────────────┐  │       │
│  │  │  Supervisor   │  │      │  │  Supervisor   │  │       │
│  │  │   ┌─────────┐ │  │      │  │   ┌─────────┐ │  │       │
│  │  │   │Worker 1 │ │  │◄────►│  │   │Worker 3 │ │  │       │
│  │  │   │Worker 2 │ │  │      │  │   │Worker 4 │ │  │       │
│  │  │   └─────────┘ │  │      │  │   └─────────┘ │  │       │
│  │  └───────────────┘  │      │  └───────────────┘  │       │
│  │                     │      │                     │       │
│  │  [ETS Tables]       │      │  [ETS Tables]       │       │
│  │  [DETS Storage]     │      │  [DETS Storage]     │       │
│  └─────────────────────┘      └─────────────────────┘       │
│                                                              │
│              Distribution Protocol (TCP/TLS)                 │
└─────────────────────────────────────────────────────────────┘
```

### Supervisor Semantics

```rust
enum RestartStrategy {
    OneForOne,   // Restart only the failed child
    OneForAll,   // Restart all children if one fails
    RestForOne,  // Restart failed child and all children after it
}

struct SupervisorSpec {
    strategy: RestartStrategy,
    max_restarts: u32,      // Max restarts allowed...
    max_seconds: u32,       // ...within this time window
    children: Vec<ChildSpec>,
}

struct ChildSpec {
    id: String,
    start: StartFn,
    restart: RestartType,   // Permanent, Temporary, Transient
    shutdown: ShutdownType, // Timeout or Brutal
}
```

### Deliverables

- [ ] `Supervisor` with restart strategies
- [ ] Supervision tree construction and management
- [ ] `Node` with distribution protocol
- [ ] Cross-node message passing
- [ ] Location-transparent process references
- [ ] Hot code loading at runtime level
- [ ] `DurableTable` (DETS-like persistence)
- [ ] Cluster membership and discovery

### Success Criteria

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

## The Erlang Parallel

For reference, mapping Erlang/OTP concepts to the Oxur ecosystem:

| Erlang/OTP | Oxur Ecosystem | Description |
|------------|----------------|-------------|
| Erlang | Oxur | The language |
| BEAM | (none — tree-walking) | Bytecode VM (we skip this) |
| OTP | Fangorn | Runtime system (processes, supervisors, distribution) |
| ERTS | Fangorn | Full runtime (scheduler, memory, ports, distribution) |
| ETS | `Table` | In-memory shared storage |
| DETS | `DurableTable` | Persistent storage |
| `gen_server` | TBD | Generic server behavior |
| `supervisor` | `Supervisor` | Process supervision |

**Key difference:** BEAM needs bytecode because Erlang can't easily compile to native code. We have `rustc` as our native compilation escape hatch, potentially making bytecode unnecessary.

---

## Design Principles

### 1. Incremental Value

Each phase delivers standalone value:
- **v1:** Usable interpreter with REPL
- **v2:** Better developer experience (hot loading, no files)
- **v3:** Concurrency for real applications
- **v4:** Production reliability

No phase depends on promises of future phases.

### 2. Rust's Strengths, Not Fighting Them

Rust's ownership model is an *asset*, not an obstacle:
- Process isolation enforced at compile time
- Zero-copy message passing where safe
- No GC pauses (unlike BEAM)
- Native performance for hot paths

### 3. BEAM's Patterns, Not BEAM's Implementation

We're not cloning BEAM. We're taking its *architectural insights*:
- Isolation enables fault tolerance
- Message passing simplifies reasoning
- Supervision enables recovery
- Distribution enables scaling

These patterns work in any language.

### 4. The Thin Layer Principle

Like LFE delegates to BEAM, Fangorn delegates to Rust:
- Type checking → `rustc`
- Optimization → LLVM
- Memory safety → Rust's borrow checker
- Native performance → Rust compilation

Fangorn provides the *operational* layer; Rust provides the *computational* layer.

---

## Prior Art & Inspiration

| Project | Relationship to Fangorn |
|---------|------------------------|
| **BEAM/OTP** | Primary inspiration for process model and supervision |
| **LFE** | Inspiration for "thin layer" principle |
| **Lunatic** | Closest existing project; worth studying deeply |
| **Bastion** | Rust actor framework with supervision |
| **Tokio** | Foundation for async I/O (potential building block) |
| **Rhai** | Patterns for tree-walking interpretation |
| **Miri** | Patterns for ownership tracking |

### Lunatic Specifically

[Lunatic](https://lunatic.solutions/) is the most mature BEAM-like runtime for Rust/WASM. Key characteristics:

- Processes as WASM instances (strong isolation)
- Message passing via serialization
- Supervision trees
- Distribution

**Relationship to Fangorn:** Lunatic could be a future compilation target, or a source of patterns. Worth deep study before v3.

---

## Open Questions

### Resolved by v1 Development

- How expensive is ownership tracking in practice?
- What's the optimal `Value` representation size?
- How much of `syn` AST do we actually need?

### Resolved by v2 Development

- What's the right granularity for incremental compilation?
- How to handle circular dependencies in hot loading?
- Background compilation threading model?

### Resolved by v3 Development

- Reduction counting: what counts as a reduction?
- Scheduler: work-stealing vs. shared queue?
- Process heap: separate allocator or standard?
- Message passing: copy always, or move when possible?

### Resolved by v4 Development

- Distribution protocol: custom or existing (e.g., QUIC)?
- Hot code loading: how to handle state migration?
- Cluster membership: static config or dynamic discovery?

---

## Timeline Speculation

**Caveat:** These are rough estimates. Each phase should be re-evaluated after the previous phase stabilizes.

| Phase | Optimistic | Expected | Pessimistic |
|-------|------------|----------|-------------|
| v1 | 4 months | 5 months | 7 months |
| v2 | 2 months | 3 months | 5 months |
| v3 | 4 months | 6 months | 10 months |
| v4 | 4 months | 8 months | 12+ months |

**Total to "telephone switch reliability":** 14-34 months from today.

---

## Conclusion

The path from Treebeard to Fangorn is clear:

1. **Interpreter** — Execute code (v1)
2. **In-memory infrastructure** — Hot loading (v2)  
3. **Processes** — Concurrency (v3)
4. **Supervision + Distribution** — Reliability (v4)

Each step builds on the previous. Nothing in v1's design blocks the later phases. The terminology evolves naturally:

- **v1-v2:** "Treebeard" — a tree-walking interpreter
- **v3-v4:** "Fangorn" — a runtime system

No bytecode required. No "VM" label needed. Just a steady evolution from interpreter to runtime, with BEAM's battle-tested patterns implemented in Rust.

---

*"The trees have grown wild and dangerous. But there is still hope. Come, I will show you the power of the forest."*

— Future Fangorn documentation, probably

---

**End of Vision Document**
