use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use reactive_language::bytecode::read_instructions_from_file;
use reactive_language::bytecode::write_instructions_to_file;
use reactive_language::compiler::{CompileContext, LabelGenerator, compile_module};
use reactive_language::grammar::Instruction;
use reactive_language::parser::parse;
use reactive_language::tokenizer::tokenize;
use reactive_language::vm::VM;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args[0].as_str() {
        "bootstrap" => {
            if args.len() != 1 {
                eprintln!("Usage: reactive bootstrap");
                std::process::exit(1);
            }
            run_bootstrap();
        }
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
            run_bootstrap_vm_entry(&input_path, &output_path, "compile_file");
        }
        "compile-module" => {
            if args.len() < 2 || args.len() > 3 {
                eprintln!("Usage: reactive compile-module <input.rx> [output.rxb]");
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
            run_bootstrap_vm_entry(&input_path, &output_path, "compile_file_module");
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

fn run_bootstrap() {
    let compiler_path = PathBuf::from("project")
        .join("bootstrap")
        .join("compiler.rx");
    let source = fs::read_to_string(&compiler_path).unwrap_or_else(|e| {
        exit_error(&format!(
            "failed to read `{}`: {}",
            compiler_path.display(),
            e
        ))
    });

    let tokens = tokenize(&source);
    let ast = parse(tokens);

    let mut bytecode: Vec<Instruction> = Vec::new();
    let mut label_gen = LabelGenerator::new();
    let mut break_stack = Vec::new();
    let mut continue_stack = Vec::new();
    let mut compile_ctx = CompileContext::new();

    compile_module(
        ast,
        &mut bytecode,
        &mut label_gen,
        &mut break_stack,
        &mut continue_stack,
        &mut compile_ctx,
    );

    let out_path = PathBuf::from("target").join("bootstrap_compiler.rxb");
    write_instructions_to_file(out_path.to_str().unwrap(), &bytecode)
        .unwrap_or_else(|e| exit_error(&format!("failed to write `{}`: {}", out_path.display(), e)));
}

fn emit_string_literal(code: &mut Vec<Instruction>, labels: &mut LabelGenerator, value: &str) {
    code.push(Instruction::Push(value.chars().count() as i32));
    code.push(Instruction::ArrayNew);

    let tmp = labels.fresh("__bootstrap_str");
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

fn compile_source(path: &Path) -> Vec<Instruction> {
    let output_path = PathBuf::from("target").join("bootstrap_out.rxb");
    run_bootstrap_vm_entry(path, &output_path, "compile_file");
    read_instructions_from_file(output_path.to_str().unwrap()).unwrap_or_else(|e| exit_error(&e))
}

fn exit_error(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn run_bootstrap_vm_entry(input_path: &Path, output_path: &Path, entry: &str) {
    let compiler_path = PathBuf::from("target").join("bootstrap_compiler.rxb");
    if !compiler_path.exists() {
        exit_error("bootstrap compiler missing: run `reactive bootstrap` first");
    }

    let mut bytecode = read_instructions_from_file(compiler_path.to_str().unwrap())
        .unwrap_or_else(|e| exit_error(&e));

    let mut arg_labels = LabelGenerator::new();
    let input_str = input_path.to_string_lossy();
    let output_str = output_path.to_string_lossy();
    emit_string_literal(&mut bytecode, &mut arg_labels, &input_str);
    emit_string_literal(&mut bytecode, &mut arg_labels, &output_str);
    bytecode.push(Instruction::Call(entry.to_string(), 2));
    bytecode.push(Instruction::Return);

    let mut vm = VM::new(bytecode);
    vm.run();
}
