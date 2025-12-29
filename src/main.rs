use std::env;
use std::path::{Path, PathBuf};

use reactive::bytecode::read_instructions_from_file;
use reactive::grammar::Instruction;
use reactive::vm::VM;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
    }

    match args[0].as_str() {
        // ------------------------------------------------------------
        // Help command
        // ------------------------------------------------------------
        "help" | "--help" | "-h" => {
            print_help();
        }
        // ------------------------------------------------------------
        // Bootstrap experimental compiler using stable compiler
        // ------------------------------------------------------------
        "bootstrap" => {
            if args.len() != 1 {
                exit_error("Usage: reactive bootstrap");
            }

            let compiler = PathBuf::from("project/bootstrap/stable/compiler.rxb");
            let input = PathBuf::from("project/bootstrap/experimental/compiler.rx");
            let output = PathBuf::from("project/bootstrap/experimental/compiler.rxb");

            run_compiler_vm_entry(&compiler, &input, &output, "compile_file_module");
        }

        // ------------------------------------------------------------
        // Compile program with stable compiler (requires main)
        // ------------------------------------------------------------
        "compile" => {
            if args.len() < 2 || args.len() > 3 {
                exit_error("Usage: reactive compile <input.rx> [output.rxb]");
            }

            let compiler = PathBuf::from("project/bootstrap/stable/compiler.rxb");
            let input = resolve_source_path(&args[1]);
            let output = output_path(&input, args.get(2));

            run_compiler_vm_entry(&compiler, &input, &output, "compile_file");
        }
        // ------------------------------------------------------------
        // Compile module with stable compiler (no main required)
        // ------------------------------------------------------------
        "compile-module" => {
            if args.len() < 2 || args.len() > 3 {
                exit_error("Usage: reactive compile-module <input.rx> [output.rxb]");
            }

            let compiler = PathBuf::from("project/bootstrap/stable/compiler.rxb");
            let input = resolve_source_path(&args[1]);
            let output = output_path(&input, args.get(2));

            run_compiler_vm_entry(&compiler, &input, &output, "compile_file_module");
        }

        // ------------------------------------------------------------
        // Compile program with expiermental compiler (requires main)
        // ------------------------------------------------------------
        "compile-expi" => {
            if args.len() < 2 || args.len() > 3 {
                exit_error("Usage: reactive compile-experimental <input.rx> [output.rxb]");
            }

            let compiler = PathBuf::from("project/bootstrap/experimental/compiler.rxb");
            let input = resolve_source_path(&args[1]);
            let output = output_path(&input, args.get(2));

            run_compiler_vm_entry(&compiler, &input, &output, "compile_file");
        }
        // ------------------------------------------------------------
        // Compile module with experimental compiler (no main required)
        // ------------------------------------------------------------
        "compile-expi-module" => {
            if args.len() < 2 || args.len() > 3 {
                exit_error("Usage: reactive compile-experimental <input.rx> [output.rxb]");
            }

            let compiler = PathBuf::from("project/bootstrap/experimental/compiler.rxb");
            let input = resolve_source_path(&args[1]);
            let output = output_path(&input, args.get(2));

            run_compiler_vm_entry(&compiler, &input, &output, "compile_file_module");
        }

        // ------------------------------------------------------------
        // Run bytecode
        // ------------------------------------------------------------
        "run" => {
            if args.len() != 2 {
                exit_error("Usage: reactive run <input.rxb>");
            }

            let code = read_instructions_from_file(&args[1]).unwrap_or_else(|e| exit_error(&e));
            VM::new(code).run();
        }

        _ => {
            exit_error("unknown command (try 'reactive help')");
        }
    }
}

// ================================================================
// Core VM compiler runner (single source of truth)
// ================================================================
fn run_compiler_vm_entry(compiler_path: &Path, input_path: &Path, output_path: &Path, entry: &str) {
    if !compiler_path.exists() {
        exit_error(&format!(
            "compiler bytecode missing: `{}`",
            compiler_path.display()
        ));
    }

    let mut bytecode = read_instructions_from_file(compiler_path.to_str().unwrap())
        .unwrap_or_else(|e| exit_error(&e));

    emit_string_literal(&mut bytecode, &input_path.to_string_lossy());
    emit_string_literal(&mut bytecode, &output_path.to_string_lossy());

    bytecode.push(Instruction::Call(entry.to_string(), 2));
    bytecode.push(Instruction::Return);

    VM::new(bytecode).run();
}

// ================================================================
// Helpers
// ================================================================
fn print_help() -> ! {
    println!(
        "Reactive Language CLI

Commands:
  bootstrap
      Build experimental compiler from stable compiler

  compile <input.rx> [output.rxb]
      Compile a program (requires main) using stable compiler

  compile-module <input.rx> [output.rxb]
      Compile a module using stable compiler (no main required)

  compile-expi <input.rx> [output.rxb]
      Compile a program using experimental compiler

  compile-expi-module <input.rx> [output.rxb]
      Compile a program using experimental compiler

  run <input.rxb>
      Run bytecode

Shortcuts:
  reactive file.rx     Compile with stable compiler and run
  reactive file.rxb    Run bytecode directly
"
    );
    std::process::exit(0);
}

fn emit_string_literal(code: &mut Vec<Instruction>, value: &str) {
    code.push(Instruction::Push(value.chars().count() as i32));
    code.push(Instruction::ArrayNew);

    let tmp = "__cli_str".to_string();
    code.push(Instruction::Store(tmp.clone()));

    for (i, ch) in value.chars().enumerate() {
        code.push(Instruction::Load(tmp.clone()));
        code.push(Instruction::Push(i as i32));
        code.push(Instruction::ArrayLValue);
        code.push(Instruction::PushChar(ch as u32));
        code.push(Instruction::StoreThrough);
    }

    code.push(Instruction::Load(tmp));
}

fn resolve_source_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(name);
    if path.extension().is_none() {
        path.set_extension("rx");
    }
    path
}

fn output_path(input: &Path, arg: Option<&String>) -> PathBuf {
    arg.map(PathBuf::from).unwrap_or_else(|| {
        let mut out = input.to_path_buf();
        out.set_extension("rxb");
        out
    })
}

fn exit_error(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
