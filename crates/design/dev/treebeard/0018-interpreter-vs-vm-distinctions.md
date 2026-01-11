# Tree-Walking Interpreter vs Virtual Machine: Key Distinctions

| Aspect | Tree-Walking Interpreter | Virtual Machine |
|--------|--------------------------|-----------------|
| **Input** | AST (tree structure) | Bytecode (flat array) |
| **Execution** | Recursive `eval()` calls | Loop with instruction pointer |
| **Control flow** | Call stack (host language's stack) | Explicit VM stack + jump instructions |
| **Compilation step** | None | AST → Bytecode |
| **Startup time** | Instant | Compilation delay |
| **Execution speed** | Slower (tree traversal overhead) | Faster (cache-friendly, no recursion) |
| **Implementation complexity** | Simpler | More complex |
| **Memory access pattern** | Pointer chasing (cache-unfriendly) | Linear array (cache-friendly) |
| **Examples** | Treebeard, early Ruby, some Lisps | BEAM, JVM, CPython, Lua, Rune |

## Summary

- **Tree-Walking Interpreter**: Directly traverses and executes the AST via recursive function calls. No intermediate representation.

- **Virtual Machine**: Compiles AST to bytecode first, then executes bytecode in a loop with an instruction pointer. The "virtual machine" simulates a CPU that natively runs the bytecode instruction set.

- **Runtime**: Umbrella term for "the environment in which code executes" — applies to both interpreters and VMs, plus any supporting infrastructure (scheduler, GC, process management, etc.).
