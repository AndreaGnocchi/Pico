# Pico

A small bytecode virtual machine and compiler for a tiny language, written in Rust.

## Goals

- A language small enough to hold in your head: 10 keywords, integers, bools and `nil`, functions and recursion.
- A compact instruction set (about 20 opcodes) executed by a stack-based VM with call frames.
- Built from scratch: lexer, parser, compiler and VM, with no parsing or VM crates.

## Status

Early development. Nothing usable yet.

## Documentation

Language and VM design notes live in [docs/DESIGN.md](docs/DESIGN.md).
Alongside them there is the documentation used to build this project [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md)

## License

See [LICENSE](LICENSE).
