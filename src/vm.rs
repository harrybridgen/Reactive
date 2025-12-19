use crate::grammar::AST;
use crate::grammar::Instruction;
use crate::grammar::Operator;
use crate::grammar::Type;
use std::collections::HashMap;

pub struct VM {
    stack: Vec<i32>,
    environment: HashMap<String, Type>,
    pointer: usize,
    code: Vec<Instruction>,
    labels: HashMap<String, usize>,
}

impl VM {
    pub fn new(code: Vec<Instruction>) -> Self {
        let mut labels = HashMap::new();
        for (i, instr) in code.iter().enumerate() {
            if let Instruction::Label(name) = instr {
                labels.insert(name.clone(), i);
            }
        }
        VM {
            stack: Vec::new(),
            environment: HashMap::new(),
            pointer: 0,
            code,
            labels,
        }
    }
    pub fn run(&mut self) {
        while self.pointer < self.code.len() {
            let instruction = &self.code[self.pointer];
            match instruction {
                Instruction::Push(n) => self.stack.push(*n),
                Instruction::Store(name) => {
                    let value = self.stack.pop().unwrap();
                    self.environment.insert(name.clone(), Type::Integer(value));
                }
                Instruction::StoreLazy(name, ast) => {
                    self.environment
                        .insert(name.clone(), Type::LazyInteger(ast.clone()));
                }
                Instruction::Load(name) => {
                    let value = self.environment.get(name).unwrap().clone();
                    match value {
                        Type::Integer(n) => self.stack.push(n),
                        Type::LazyInteger(ast) => {
                            let result = self.evaluate(*ast);
                            self.stack.push(result);
                        }
                    }
                }
                Instruction::Add => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(b + a);
                }
                Instruction::Mul => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(b * a);
                }
                Instruction::Div => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(b / a);
                }
                Instruction::Sub => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(b - a);
                }
                Instruction::Print => {
                    let value = self.stack.pop().unwrap();
                    println!("{value}");
                }
                Instruction::Greater => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b > a {
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                Instruction::Less => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b < a {
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                Instruction::Equal => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b == a {
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                Instruction::Or => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b > 0 || a > 0 {
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                Instruction::And => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b > 0 && a > 0 {
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                Instruction::Label(_) => {}
                Instruction::Jump(str) => {
                    self.pointer = *self.labels.get(str).unwrap();
                    continue;
                }
                Instruction::JumpIfZero(str) => {
                    let value = self.stack.pop();
                    if value == Some(0) {
                        self.pointer = *self.labels.get(str).unwrap();
                        continue;
                    }
                }
                Instruction::GreaterEqual => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b > a || a == b{
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                Instruction::LessEqual => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b < a || a == b{
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                Instruction::NotEqual => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if  a != b{
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
            }
            //self.dump(instruction);
            self.pointer += 1;
        }
    }

    fn evaluate(&self, ast: AST) -> i32 {
        match ast {
            AST::Number(n) => n,
            AST::Var(name) => {
                let var = self.environment.get(&name).unwrap().clone();
                match var {
                    Type::Integer(n) => n,
                    Type::LazyInteger(ast) => self.evaluate(*ast),
                }
            }
            AST::Operation(left, op, right) => {
                let l = self.evaluate(*left);
                let r = self.evaluate(*right);

                match op {
                    Operator::Addition => l + r,
                    Operator::Multiplication => l * r,
                    Operator::Division => l / r,
                    Operator::Subtraction => l - r,
                    Operator::Greater => {
                        if l > r {
                            1
                        } else {
                            0
                        }
                    }
                    Operator::Less => {
                        if l < r {
                            1
                        } else {
                            0
                        }
                    }
                    Operator::GreaterEqual => {
                        if l > r || l == r {
                            1
                        } else {
                            0
                        }
                    }
                    Operator::LessEqual => {
                        if l < r || l == r  {
                            1
                        } else {
                            0
                        }
                    }
                    Operator::NotEqual => {
                        if l != r {
                            1
                        } else {
                            0
                        }
                    }
                    Operator::Equal => {
                        if l == r {
                            1
                        } else {
                            0
                        }
                    }
                    Operator::Or => {
                        if l > 0 || r > 0 {
                            1
                        } else {
                            0
                        }
                    }
                    Operator::And => {
                        if l > 0 && r > 0 {
                            1
                        } else {
                            0
                        }
                    }
                }
            }
            _ => panic!("Error in AST evaluator"),
        }
    }

    pub fn dump(&self, instr: &Instruction) {
        println!(
            "{:<20} {:<55} {:?}",
            format!("{:?}", self.stack),
            format!("{:?}", instr),
            self.environment
        );
    }
}
