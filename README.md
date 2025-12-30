# Reactive Language

Reactive is a small, expression-oriented language compiled to bytecode and executed on a stack-based VM.
The Rust VM lives in `src/`, and a bootstrapped compiler lives in `project/bootstrap/`.

## Quick Start

#### Build the VM

In root folder `reactive`.
Run:

```powershell
cargo build
```

#### Compiling and Running programs

In root folder `reactive`.
Run the compiler without changing PATH:

```powershell
cargo run -- compile myproject/main.rx
cargo run -- run myproject/main.rxb
```

#### Adding to PATH

In root folder `reactive`.

Add the binary to your PATH (Windows PowerShell):

```powershell
$env:Path = "$env:Path;${PWD}\target\debug"
```

Or you can run:

```powershell
cargo install --path .
```

Then you can run:

```powershell
reactive compile project/main.rx
reactive run project/main.rxb
```

Without needing to type `cargo run --`.

## VS Code Extension

See [rx-vscode/README.md](rx-vscode/README.md) for the extension docs and setup.

## Documentation

- Language guide: [docs/language-guide.md](docs/language-guide.md)
- Standard library: [docs/stdlib.md](docs/stdlib.md)
- Examples: [docs/examples.md](docs/examples.md)
- Grammar reference: [docs/grammar.md](docs/grammar.md)
- Implementation notes: [docs/implementation.md](docs/implementation.md)
