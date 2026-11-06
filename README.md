# Pico

A small bytecode compiler and stack-based virtual machine for a tiny language, written from scratch in Rust.

It reads a source file, lexes, parses and compiles it to bytecode, and runs it on the VM. There are no parsing or VM crates, and no dependencies at all.

## About

This is a small learning project. It was built to understand how lexers and parsers work, not to design a language. Pico itself is deliberately minimal and is not meant to be useful, complete or taken seriously as a language.

The compiler and VM exist so the parser has something to feed into and the whole pipeline can be run end to end. They are kept simple on purpose.

## Usage

Write a program:

```
// comment
let x = 10;
x = x + 1;

fn fib(n) {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);
}

let i = 0;
while i < 10 {
    print fib(i);
    i = i + 1;
}
```

Run it:

```bash
cargo run -- code.pico
```

Or build a release binary and use it directly:

```bash
cargo build --release
./target/release/pico code.pico
```

This prints the first ten Fibonacci numbers, one per line, from `0` to `34`. The Rust edition is 2024, so you need Rust 1.85 or newer.

Errors from every stage are printed to stderr with a readable message and the process exits with a failure code:

| Exit code | Meaning                                                      |
|-----------|--------------------------------------------------------------|
| 0         | The program ran to completion                                |
| 1         | Missing argument, or a lexer, parse, compiler or runtime error |
| 2         | The source file could not be read                            |

## How it works

The source goes through four stages, each in its own module.

| Stage    | File                                 | What it does                                                                |
|----------|--------------------------------------|-----------------------------------------------------------------------------|
| Lexer    | [src/lexer.rs](src/lexer.rs)         | Turns the source into tokens, each with a `line:col` position               |
| Parser   | [src/parser.rs](src/parser.rs)       | Recursive descent parser that builds the AST from the grammar               |
| Compiler | [src/compiler.rs](src/compiler.rs)   | Single pass over the AST that emits opcodes and a constant pool             |
| VM       | [src/vm.rs](src/vm.rs)               | Executes the opcodes on a value stack with call frames                      |

**Values.** Dynamically typed: 64-bit integers, bools, `nil` and functions. Only `false` and `nil` are falsy.

**Lexer.** An iterator over tokens. It skips whitespace and `//` comments, and reports unexpected characters and invalid numbers with their position.

**Parser.** One function per grammar rule, with the usual precedence levels from `||` down to calls. The full EBNF is in [docs/DESIGN.md](docs/DESIGN.md).

**Compiler.** Globals are resolved to numbered slots and locals to stack slots, with block scoping. Jumps are emitted with a placeholder and patched once the target is known. `&&` and `||` short-circuit through `JumpIfFalse`, and `!=`, `>` and `>=` are derived from `Eq`, `Lt` and `Le`, so the instruction set stays small. Functions are compiled inline and referenced as `Function { addr, arity }` constants.

**Instruction set.**

| Group      | Opcodes                                                           |
|------------|-------------------------------------------------------------------|
| Constants  | `Const`, `True`, `False`, `Nil`                                   |
| Stack      | `Pop`                                                             |
| Variables  | `GetLocal`, `SetLocal`, `GetGlobal`, `SetGlobal`                  |
| Arithmetic | `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Neg`                          |
| Logic      | `Not`, `Eq`, `Lt`, `Le`                                           |
| Control    | `Jump`, `JumpIfFalse`, `Loop` (all relative offsets)              |
| Functions  | `Call`, `Ret`, `Print`                                            |

**VM.** Slot 0 of the stack is a `nil` placeholder for the top level. A call pushes a frame whose base is the callee itself, so locals are addressed relative to that base and arguments are already in place. `Ret` truncates the stack back to the base and pushes the result. Arithmetic is checked: overflow, including `i64::MIN / -1` and negating `i64::MIN`, is a runtime error rather than a wrap or a panic. Division truncates toward zero and the remainder takes the sign of the dividend. Call depth is capped at 1024 frames.

**Runtime errors.** Every runtime error records the instruction pointer of the failing instruction. The kinds are stack underflow, type error, division by zero, integer overflow, bad constant, global, local or jump target, undefined global, not callable, arity mismatch and stack overflow.

## Known limitations

- No closures, and functions can only be declared at top level.
- `return` outside a function is a compile error.
- Only integers, bools and `nil`. There are no strings, floats or arrays.
- Integer overflow is a runtime error, not a wraparound.
- Runtime errors report an instruction index, not a source line.
- Reading an undefined global is a runtime error, not a compile error.
- No optimizations and no bytecode disassembler.
- `print` writes to stdout and there is no other I/O. The input is always a file, there is no REPL.

## Documentation

Language and VM design notes live in [docs/DESIGN.md](docs/DESIGN.md).
Alongside them there is the documentation used to build this project [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md).

---
# Tree
``` bash
Pico
├── docs
│   ├── DESIGN.md
│   └── DOCUMENTATION.md
├── .github
│   └── workflows
│       └── ci.yml
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── code.pico
├── LICENSE
├── README.md
└── src
    ├── compiler.rs
    ├── lexer.rs
    ├── main.rs
    ├── opcode.rs
    ├── parser.rs
    └── vm.rs
```
# Build

```bash
# Compile in debug mode
cargo build

# Compile an optimized binary
cargo build --release

# Clean all build artifacts
cargo clean
```

# Tests

```bash
# Run the unit tests of every stage
cargo test

# Run clippy, treating warnings as errors
cargo clippy --all-targets -- -D warnings
```

The suite has 182 tests: 39 for the lexer, 37 for the parser, 30 for the compiler and 76 for the VM. The VM tests build programs from raw opcodes and check the values left on the stack and the exact runtime errors, including integer overflow edge cases, jump bounds, call arity and stack overflow. CI runs `cargo test` on every push to `main` and on pull requests.

---
# License
See [LICENSE](LICENSE).
