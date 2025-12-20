use crate::grammar::AST;
use crate::grammar::Instruction;
use crate::grammar::Operator;
use crate::grammar::Type;
use std::collections::HashMap;

pub struct VM {
    stack: Vec<Type>,
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
            let instruction = self.code[self.pointer].clone();


            match instruction {
                Instruction::Push(n) => self.stack.push(Type::Integer(n)),

                Instruction::Store(name) => {
                    let value = self.stack.pop().expect("Stack underflow on Store");
                    self.environment.insert(name.clone(), value);
                }

                Instruction::StoreLazy(name, ast) => {
                    self.environment
                        .insert(name.clone(), Type::LazyInteger(ast.clone()));
                }

                Instruction::Load(name) => {
                    let value = self
                        .environment
                        .get(&name)
                        .unwrap_or_else(|| panic!("Undefined variable: {name}"))
                        .clone();
                    let loaded = self.load_value(value);
                    self.stack.push(loaded);
                }

                Instruction::Add => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(b + a));
                }


                Instruction::Mul => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(b * a));
                }

                Instruction::Div => {
                                       let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(b / a));
                }

                Instruction::Sub => {
                                       let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(b - a));
                }

                Instruction::Print => {
                    let v = self.stack.pop().expect("Stack underflow on Print");
                    let n = self.as_int(v);
                    print!("{n}");
                }

                Instruction::Println => {
                    let v = self.stack.pop().expect("Stack underflow on Println");
                    let n = self.as_int(v);
                    println!("{n}");
                }

                Instruction::Greater => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(if b > a { 1 } else { 0 }));
                }

                Instruction::Less => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(if b < a { 1 } else { 0 }));
                }

                Instruction::Equal => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(if b == a { 1 } else { 0 }));
                }

                Instruction::Or => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack
                        .push(Type::Integer(if b > 0 || a > 0 { 1 } else { 0 }));
                }

                Instruction::And => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack
                        .push(Type::Integer(if b > 0 && a > 0 { 1 } else { 0 }));
                }

                Instruction::GreaterEqual => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack
                        .push(Type::Integer(if b > a || b == a { 1 } else { 0 }));
                }

                Instruction::LessEqual => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack
                        .push(Type::Integer(if b < a || b == a { 1 } else { 0 }));
                }

                Instruction::NotEqual => {
                    let a = self.pop_int("Stack underflow on Add (a)");
                    let b = self.pop_int("Stack underflow on Add (b)");
                    self.stack.push(Type::Integer(if b != a { 1 } else { 0 }));
                }

                Instruction::Label(_) => {}

                Instruction::Jump(label) => {
                    self.pointer = *self
                        .labels
                        .get(&label)
                        .unwrap_or_else(|| panic!("Unknown label: {label}"));
                    continue;
                }

                Instruction::JumpIfZero(label) => {
                    let value = self.pop_int("Stack underflow on Add (a)");
                    if value == 0 {
                        self.pointer = *self
                            .labels
                            .get(&label)
                            .unwrap_or_else(|| panic!("Unknown label: {label}"));
                        continue;
                    }
                }

                Instruction::ArrayNew => {
                    let size_v = self.stack.pop().expect("Stack underflow on ArrayNew");
                    let size = self.as_int(size_v);
                    if size < 0 {
                        panic!("Array size cannot be negative");
                    }

                    let mut items = Vec::with_capacity(size as usize);
                    for _ in 0..(size as usize) {
                        items.push(Type::Integer(0));
                    }

                    self.stack.push(Type::Array(items));
                }

                Instruction::ArrayGet => {
                    let index_v = self.stack.pop().expect("Stack underflow on ArrayGet (index)");
                    let array_v = self.stack.pop().expect("Stack underflow on ArrayGet (array)");

                    let idx = self.as_int(index_v);
                    if idx < 0 {
                        panic!("Array index cannot be negative");
                    }

                    match array_v {
                        Type::Array(items) => {
                            let elem = items
                                .get(idx as usize)
                                .unwrap_or_else(|| panic!("Index out of bounds"))
                                .clone();
                            let elem = self.load_value(elem);
                            self.stack.push(elem);
                        }
                        other => panic!("Tried to index non-array value: {:?}", other),
                    }
                }

                Instruction::StoreIndex(name) => {
                    let value = self.stack.pop().expect("Stack underflow on StoreIndex (value)");
                    let index_v = self.stack.pop().expect("Stack underflow on StoreIndex (index)");
                    let idx = self.as_int(index_v);

                    if idx < 0 {
                        panic!("Array index cannot be negative");
                    }

                    let entry = self
                        .environment
                        .get_mut(&name)
                        .unwrap_or_else(|| panic!("Undefined variable: {name}"));

                    match entry {
                        Type::Array(items) => {
                            let i = idx as usize;
                            if i >= items.len() {
                                panic!("Index out of bounds");
                            }
                            items[i] = value;
                        }
                        other => panic!("Tried to index-assign into non-array value: {:?}", other),
                    }
                }

                Instruction::StoreIndexLazy(name, ast) => {
                    let index_v =
                        self.stack.pop().expect("Stack underflow on StoreIndexLazy (index)");
                    let idx = self.as_int(index_v);

                    if idx < 0 {
                        panic!("Array index cannot be negative");
                    }

                    let entry = self
                        .environment
                        .get_mut(&name)
                        .unwrap_or_else(|| panic!("Undefined variable: {name}"));

                    match entry {
                        Type::Array(items) => {
                            let i = idx as usize;
                            if i >= items.len() {
                                panic!("Index out of bounds");
                            }
                            items[i] = Type::LazyInteger(ast.clone());
                        }
                        other => panic!("Tried to lazy index-assign into non-array value: {:?}", other),
                    }
                }
            }

            // self.dump(instruction);
            self.pointer += 1;
        }
    }

    fn evaluate(&self, ast: AST) -> i32 {
        match ast {
            AST::Number(n) => n,

            AST::Var(name) => {
                let var = self
                    .environment
                    .get(&name)
                    .unwrap_or_else(|| panic!("Undefined variable: {name}"))
                    .clone();

                match var {
                    Type::Integer(n) => n,
                    Type::LazyInteger(ast) => self.evaluate(*ast),
                    Type::Array(items) => items.len() as i32, 
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
                        if l > r { 1 } else { 0 }
                    }
                    Operator::Less => {
                        if l < r { 1 } else { 0 }
                    }
                    Operator::GreaterEqual => {
                        if l > r || l == r { 1 } else { 0 }
                    }
                    Operator::LessEqual => {
                        if l < r || l == r { 1 } else { 0 }
                    }
                    Operator::NotEqual => {
                        if l != r { 1 } else { 0 }
                    }
                    Operator::Equal => {
                        if l == r { 1 } else { 0 }
                    }
                    Operator::Or => {
                        if l > 0 || r > 0 { 1 } else { 0 }
                    }
                    Operator::And => {
                        if l > 0 && r > 0 { 1 } else { 0 }
                    }
                }
            }
            AST::Index(base, index) => {
                let base_val = match *base {
                    AST::Var(name) => self
                        .environment
                        .get(&name)
                        .unwrap_or_else(|| panic!("Undefined variable: {name}"))
                        .clone(),
                    other => panic!("Index base must be a variable in lazy eval: {:?}", other),
                };

                let idx = self.evaluate(*index);
                if idx < 0 {
                    panic!("Array index cannot be negative");
                }

                match base_val {
                    Type::Array(items) => {
                        let elem = items
                            .get(idx as usize)
                            .unwrap_or_else(|| panic!("Index out of bounds"))
                            .clone();

                        match elem {
                            Type::Integer(n) => n,
                            Type::LazyInteger(ast) => self.evaluate(*ast),
                            Type::Array(items) => items.len() as i32, 
                        }
                    }
                    other => panic!("Tried to index non-array value in lazy eval: {:?}", other),
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

    fn as_int(& self, v: Type) -> i32 {
        match v {
            Type::Integer(n) => n,
            Type::LazyInteger(ast) => self.evaluate(*ast),
            Type::Array(items) => items.len() as i32,
        }
    }

    fn load_value(& self, v: Type) -> Type {
        match v {
            Type::LazyInteger(ast) => Type::Integer(self.evaluate(*ast)),
            other => other,
        }
    }

    fn pop(&mut self, msg: &str) -> Type {
    self.stack.pop().unwrap_or_else(|| panic!("{msg}"))
}

fn pop_int(&mut self, msg: &str) -> i32 {
    let v = self.pop(msg);
    self.as_int(v)
}
}
