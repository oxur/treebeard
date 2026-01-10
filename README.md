# 🌳 Treebeard

**A Tree-Walking Interpreter for Rust's `syn` AST**

> "I am not altogether on anybody's side, because nobody is altogether on my side."
> — Treebeard, speaking on language-agnostic design

---

## What is Treebeard?

Treebeard is a **language-agnostic execution engine** that interprets Rust's `syn` AST directly. Any language that can compile to Rust AST (Lisp, ML, Python-like syntax, etc.) can leverage Treebeard for:

- 🚀 **Instant execution** - No compilation delay
- 🔄 **REPL environments** - Interactive development
- 🎯 **Rapid iteration** - Test ideas immediately
- ⚡ **Compilation escape hatch** - Compile hot paths on demand

## The Big Idea

Most languages that target Rust go through multiple translation layers:

```
Your Language → Custom IR → More IRs → Rust → rustc → Binary
```

Treebeard takes inspiration from LFE (Lisp Flavoured Erlang) and does ONE thing well:

```
Your Language → syn AST → Treebeard Interpreter
                    ↓
                  rustc (when you need speed)
```

**The "Thin Layer" Principle:** Like LFE delegates everything to BEAM, Treebeard delegates type checking and optimization to `rustc`. This keeps the codebase under 15K lines while achieving 100% Rust interoperability.

## Current Status

🚧 **Work in Progress** - Architecture finalized, implementation underway.

### What Works

- ✅ Architecture validated (v3)
- ✅ Design documents complete
- ✅ Workspace structure created
- ✅ Integration plan with Oxur defined

### What's Next

See [Implementation Timeline](#implementation-timeline) below.

## Architecture

### Core Components

```
┌──────────────────────────────────────────────────────────┐
│                     Your Language                        │
│  (Oxur, or any language producing syn AST)               │
│                                                          │
│  Implements: LanguageFrontend trait                      │
│    - parse(source) → Vec<syn::Item>                      │
│    - expand_macros(items) → Vec<syn::Item>               │
│    - format_error(error) → String                        │
└─────────────────────┬────────────────────────────────────┘
                      │
                      │ syn AST
                      ↓
┌──────────────────────────────────────────────────────────┐
│                    Treebeard Core                        │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Evaluator        - Interprets syn AST              │  │
│  │ Environment      - Variable bindings               │  │
│  │ Value            - Runtime values                  │  │
│  │ OwnershipTracker - Runtime ownership checking      │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Features:                                               │
│  • Tree-walking interpretation                           │
│  • Ownership tracking (move/borrow semantics)            │
│  • Compilation escape hatch (hot path → rustc)           │
└──────────────────────────────────────────────────────────┘
```

### The `LanguageFrontend` Trait

Any language can plug into Treebeard by implementing this trait:

```rust
pub trait LanguageFrontend {
    /// Parse source into syn AST items
    fn parse(&self, source: &str) -> Result<Vec<syn::Item>>;

    /// Expand macros in context of environment
    fn expand_macros(
        &self,
        items: Vec<syn::Item>,
        macro_env: &MacroEnvironment
    ) -> Result<(Vec<syn::Item>, MacroEnvironment)>;

    /// Format an evaluation error for display
    fn format_error(&self, error: &EvalError, source: &str) -> String;

    /// Language metadata
    fn name(&self) -> &str;
    fn file_extension(&self) -> &str;
}
```

## Why syn AST?

1. **Ecosystem Standard** - 20,000+ crates depend on `syn`
2. **Well Documented** - Comprehensive docs for every node type
3. **Round-Trip Capable** - Can convert back to source
4. **Direct Compilation** - `syn` → `TokenStream` → `rustc`
5. **No Translation Layer** - What you interpret is what you compile

## Key Design Decisions

### ✅ Tree-Walking (Not Bytecode)

- **Simpler**: No bytecode compilation step
- **Faster startup**: No compile-time overhead
- **Better errors**: Direct mapping to source
- **Suitable for**: REPL workloads with <10K lines of hot code

### ✅ Runtime Ownership Tracking

- Catch use-after-move at runtime
- Catch double-borrow violations
- Defer complex lifetime analysis to compilation
- **Cost**: 8 bytes per value (acceptable for REPL)

### ✅ Compilation Escape Hatch

- Profile-guided: Detect hot functions (>100 calls)
- Background compilation: Don't block REPL
- Incremental: Only compile changed code
- **Speedup**: 10-100x for numeric hot paths

## Implementation Timeline

**Total: ~16-20 weeks to production-ready system**

| Phase | Duration | Deliverable | Milestone |
|-------|----------|-------------|-----------|
| **Phase 1** | 4 weeks | Core Evaluator MVP | Execute basic Rust expressions |
| **Phase 2** | 2 weeks | Frontend Trait | Language abstraction working |
| **Phase 3** | 3 weeks | Oxur Macro System | Full macro expansion |
| **Phase 4** | 2 weeks | REPL Integration | Interactive Oxur REPL |
| **Phase 5** | 3 weeks | Closures & Ownership | Full borrow checking |
| **Phase 6** | 3 weeks | Compilation Escape | Hot path optimization |
| **Phase 7** | 2 weeks | Crate Loading | External dependencies |

### Useful Milestones

- **Week 4**: Can evaluate `1 + 2 * 3` and call functions
- **Week 8**: Macros work, can define new syntax
- **Week 12**: Full REPL with history and completion
- **Week 16**: Production-ready with compilation escape

## Building and Testing

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build documentation
cargo doc --workspace --open
```

## Project Structure

```
treebeard/
├── crates/
│   ├── treebeard/         # Core interpreter
│   │   ├── src/
│   │   │   ├── evaluator.rs    # syn AST interpreter
│   │   │   ├── value.rs        # Runtime value types
│   │   │   ├── environment.rs  # Variable bindings
│   │   │   ├── ownership.rs    # Runtime ownership tracking
│   │   │   └── error.rs        # Error types
│   │   └── Cargo.toml
│   │
│   └── design/            # Design documentation
│       ├── docs/
│       │   ├── architecture.md
│       │   └── implementation-guide.md
│       └── Cargo.toml
│
├── Cargo.toml             # Workspace config
└── README.md              # This file
```

## Philosophy

### The LFE Pattern

[LFE](https://lfe.io) (Lisp Flavoured Erlang) succeeds by doing ONE thing well: syntax transformation. It compiles directly to Erlang AST and delegates everything else to BEAM.

Treebeard follows the same pattern:
- **One job**: Interpret `syn` AST
- **Delegate**: Type checking and optimization to `rustc`
- **Result**: Small codebase (<15K LOC) with full Rust interoperability

### Why This Matters

Complex VMs try to do everything:
- Custom type systems
- Custom optimization passes
- Custom memory management
- Years of development

Treebeard leverages Rust's existing infrastructure:
- ✅ Type system via `rustc`
- ✅ Optimization via LLVM
- ✅ Safety via borrow checker
- ✅ Ecosystem via Cargo

## Integration with Oxur

Treebeard was designed for [Oxur](https://github.com/oxur/oxur), a Lisp that treats Rust as its compilation target. The integration:

```
┌─────────────────────────────────────────────────────────┐
│ Oxur (oxur-vm)                                          │
│   oxur-reader → oxur-macros → oxur-ast-bridge          │
│                                      ↓                  │
│                           Implements LanguageFrontend   │
└─────────────────────────────┬───────────────────────────┘
                              │
                              ↓ syn AST
┌─────────────────────────────────────────────────────────┐
│ Treebeard                                               │
│   Core Interpreter + REPL + Compilation Escape          │
└─────────────────────────────────────────────────────────┘
```

Oxur's existing AST bridge (95% complete) provides the `LanguageFrontend` implementation.

## Contributing

**Status**: Early development - architecture is solid, implementation needs contributors!

Areas where help is needed:
- Core evaluator implementation
- `syn` AST node coverage
- REPL features
- Documentation
- Example frontends for other languages

## Prior Art and Research

Treebeard's design is informed by:

- **LFE** - Thin layer principle, delegate to existing VM
- **Rhai** - Embedded scripting with Rust integration
- **evcxr** - Rust REPL, subprocess execution model
- **Chez Scheme** - Fast tree-walking interpreter design
- **Tree-sitter** - AST representation and navigation

See `crates/design/docs/research.md` for detailed analysis.

## License

Apache-2.0

---

<div align="center">

**Built with 🦀 Rust**

*For languages that grow like trees, not rust like iron*

[Documentation](https://github.com/oxur/treebeard/tree/main/crates/design/docs) •
[Architecture](https://github.com/oxur/treebeard/blob/main/crates/design/docs/architecture.md) •
[Oxur Project](https://github.com/oxur/oxur)

</div>
