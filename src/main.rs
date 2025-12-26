use std::fs;
use std::io::{self, Write};

use reactive_language::compiler::{LabelGenerator, compile};
use reactive_language::grammar::Instruction;
use reactive_language::parser::parse;
use reactive_language::tokenizer::tokenize;
use reactive_language::vm::VM;

fn main() {
    print!("Enter file name (relative to root/project/, .rx optional, nothing for main): ");
    io::stdout().flush().unwrap();

    let mut input_name = String::new();
    io::stdin().read_line(&mut input_name).unwrap();
    let mut name = input_name.trim().to_string();

    if name.is_empty() {
        name = "main".to_string();
    }

    if !name.ends_with(".rx") {
        name.push_str(".rx");
    }

    let file_path = format!("project/{}", name);

    let input = fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("failed to read `{}`: {}", file_path, e));

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

    let mut vm = VM::new(bytecode);
    vm.run();
}
