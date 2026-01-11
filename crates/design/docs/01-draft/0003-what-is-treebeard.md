---
number: 3
title: "What is Treebeard?"
author: "recursively traversing"
component: All
tags: [change-me]
created: 2026-01-11
updated: 2026-01-11
state: Draft
supersedes: null
superseded-by: null
version: 1.0
---

# What is Treebeard?

**Treebeard is a tree-walking interpreter for Rust's `syn` AST.**

It directly executes `syn` AST nodes by recursively traversing them and computing results—without ever invoking `rustc`.

## What We Have To Build

Treebeard requires three core components: a value representation (runtime types for Rust values), an environment (variable and function bindings), and an evaluator (the recursive AST walker shown below). That's it—roughly 10k lines of Rust.

The payoff: any frontend that can produce syn AST gets a working interpreter for free.

## How It Works

Here's the basic mechanism:

```rust
fn eval_expr(expr: &syn::Expr, env: &mut Environment) -> Value {
    match expr {
        syn::Expr::Lit(lit) => {
            // Literal: just convert to Value
            match &lit.lit {
                syn::Lit::Int(i) => Value::I64(i.base10_parse().unwrap()),
                syn::Lit::Bool(b) => Value::Bool(b.value),
                // ...
            }
        }
        syn::Expr::Binary(bin) => {
            // Binary op: evaluate both sides, apply operator
            let left = eval_expr(&bin.left, env);
            let right = eval_expr(&bin.right, env);
            match bin.op {
                syn::BinOp::Add(_) => left + right,
                syn::BinOp::Mul(_) => left * right,
                // ...
            }
        }
        syn::Expr::Path(path) => {
            // Variable reference: look up in environment
            env.lookup(&path.path.segments[0].ident)
        }
        syn::Expr::Call(call) => {
            // Function call: look up function, evaluate args, execute body
            let func = eval_expr(&call.func, env);
            let args: Vec<Value> = call.args.iter()
                .map(|a| eval_expr(a, env))
                .collect();
            apply_function(func, args, env)
        }
        // ... ~50 more expression types
    }
}
```

**The interpreter walks the AST tree, node by node, evaluating as it goes.** No machine code is generated. It's like a very sophisticated calculator that understands Rust syntax.

**`rustc` is only used for the optional "escape hatch"**—when a function is called thousands of times and you want native speed, you can explicitly compile it. But the default path is pure interpretation.

This is why it's 10-100x slower than native Rust, but also why it has instant startup and supports hot reload trivially.

## The Big Picture

**Treebeard = Python-style interpreter, but for Rust's AST**

The stack looks like this:

```
┌──────────────────────────────────────┐
│   Surface Syntax                     │
│   (Rust source OR Oxur S-expressions)│
└──────────────────┬───────────────────┘
                   │ parsing (syn::parse or Oxur reader)
                   ▼
┌──────────────────────────────────────┐
│   syn AST                            │
│   (the "bytecode" equivalent)        │
└──────────────────┬───────────────────┘
                   │ tree-walking
                   ▼
┌──────────────────────────────────────┐
│   Treebeard Interpreter              │
│   (the "Python VM" equivalent)       │
└──────────────────────────────────────┘
```

## Why This Matters

**We're not inventing a new language—we're making Rust's existing AST able to be evaluated without compilation.**

Anyone who can produce `syn` AST can use the treebeard interpreter for their own language:

- **Oxur** → S-expressions that become `syn` AST
- **Rust itself** → `syn::parse_str("fn foo() { ... }")`
- **Any DSL** → Custom syntax → `syn` AST
- **Proc-macro authors** → Test macro output interactively

It's wild when you think about it. Python has `eval()`. JavaScript has `eval()`. Rust has... nothing like that. **Treebeard fills that gap.**

## The Trade-off

The trade-off is real: no type checking, no borrow checking (except our simplified runtime version), no optimizations. You're running "unverified" Rust.

But for a REPL, exploratory programming, and rapid iteration—**that's exactly what you want.**
