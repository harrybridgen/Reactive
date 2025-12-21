use std::fs;

use reactive_language::compiler::compile;
use reactive_language::compiler::LabelGenerator;
use reactive_language::grammar::Instruction;
use reactive_language::parser::parse;
use reactive_language::tokenizer::tokenize;
use reactive_language::vm::VM;

fn main() {
    let file_path = String::from("project/main.rx");

    let input = fs::read_to_string(file_path).expect("Could not read file");

    let tokens = tokenize(&input);
    println!("{:?}", tokens);

    let ast = parse(tokens);
    println!("{:#?}", ast);

    let mut byte_code: Vec<Instruction> = Vec::new();
    let mut label_gen = LabelGenerator::new();
    let mut break_stack = Vec::new();

    compile(ast, &mut byte_code, &mut label_gen, &mut break_stack);

    println!("{:?}", byte_code);

    let mut vm = VM::new(byte_code);
    vm.run();
}
