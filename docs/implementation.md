# Implementation Notes

- Rust VM and CLI live in `src/`.
- Bootstrapped compiler sources live in `project/bootstrap/`.
- Bytecode is serialized to a text format with an `RXB1` header.
- Imports load and execute modules once per program run.

Entry points:

- `reactive compile <input.rx> [output.rxb]`
- `reactive compile-module <input.rx> [output.rxb]`
- `reactive run <input.rxb>`
