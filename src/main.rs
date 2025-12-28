use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use reactive_language::bytecode::{read_instructions_from_file, write_instructions_to_file};
use reactive_language::compiler::{LabelGenerator, compile};
use reactive_language::grammar::Instruction;
use reactive_language::parser::parse;
use reactive_language::tokenizer::tokenize;
use reactive_language::vm::VM;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        interactive_run();
        return;
    }

    match args[0].as_str() {
        "compile" => {
            if args.len() < 2 || args.len() > 3 {
                eprintln!("Usage: reactive compile <input.rx> [output.rxb]");
                std::process::exit(1);
            }
            let input_path = resolve_source_path(&args[1]);
            ensure_extension(&input_path, "rx").unwrap_or_else(|e| exit_error(&e));
            let output_path = if args.len() == 3 {
                PathBuf::from(&args[2])
            } else {
                let mut out = input_path.clone();
                out.set_extension("rxb");
                out
            };
            let code = compile_source(&input_path);
            write_instructions_to_file(output_path.to_str().unwrap(), &code)
                .unwrap_or_else(|e| exit_error(&format!("failed to write bytecode: {}", e)));
        }
        "run" => {
            if args.len() != 2 {
                eprintln!("Usage: reactive run <input.rxb>");
                std::process::exit(1);
            }
            let bytecode_path = resolve_bytecode_path(&args[1]);
            ensure_extension(&bytecode_path, "rxb").unwrap_or_else(|e| exit_error(&e));
            let code = read_instructions_from_file(bytecode_path.to_str().unwrap())
                .unwrap_or_else(|e| exit_error(&e));
            let mut vm = VM::new(code);
            vm.run();
        }
        other => {
            if other.ends_with(".rxb") {
                let bytecode_path = resolve_bytecode_path(other);
                let code = read_instructions_from_file(bytecode_path.to_str().unwrap())
                    .unwrap_or_else(|e| exit_error(&e));
                let mut vm = VM::new(code);
                vm.run();
            } else {
                let input_path = resolve_source_path(other);
                let code = compile_source(&input_path);
                let mut vm = VM::new(code);
                vm.run();
            }
        }
    }
}

fn interactive_run() {
    print!("Enter file name (relative to root/project/, .rx optional, nothing for main): ");
    io::stdout().flush().unwrap();

    let mut input_name = String::new();
    io::stdin().read_line(&mut input_name).unwrap();
    let mut name = input_name.trim().to_string();

    if name.is_empty() {
        name = "main".to_string();
    }

    let input_path = resolve_source_path(&name);
    let code = compile_source(&input_path);
    let mut vm = VM::new(code);
    vm.run();
}

fn resolve_source_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(name);
    if path.components().count() == 1 {
        path = PathBuf::from("project").join(path);
    }
    if path.extension().is_none() {
        path.set_extension("rx");
    }
    path
}

fn resolve_bytecode_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(name);
    if path.components().count() == 1 {
        path = PathBuf::from("project").join(path);
    }
    if path.extension().is_none() {
        path.set_extension("rxb");
    }
    path
}

fn ensure_extension(path: &Path, expected: &str) -> Result<(), String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext == expected => Ok(()),
        Some(ext) => Err(format!(
            "expected .{} file, got .{} for `{}`",
            expected,
            ext,
            path.display()
        )),
        None => Err(format!(
            "expected .{} file, got `{}`",
            expected,
            path.display()
        )),
    }
}

fn compile_source(path: &Path) -> Vec<Instruction> {
    let input = fs::read_to_string(path)
        .unwrap_or_else(|e| exit_error(&format!("failed to read `{}`: {}", path.display(), e)));
    let tokens = tokenize(&input);
    let ast = parse(tokens);

    let mut bytecode: Vec<Instruction> = Vec::new();
    let mut label_gen = LabelGenerator::new();
    let mut break_stack = Vec::new();
    let mut continue_stack = Vec::new();
    compile(
        ast,
        &mut bytecode,
        &mut label_gen,
        &mut break_stack,
        &mut continue_stack,
    );
    bytecode
}

fn exit_error(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
