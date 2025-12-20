use std::fs;

use simple_parser::compiler::compile;
use simple_parser::compiler::LabelGenerator;
use simple_parser::grammar::Instruction;
use simple_parser::parser::parse;
use simple_parser::tokenizer::tokenize;
use simple_parser::vm::VM;

fn main() {
    let file_path = String::from("programs/test.hs");

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
