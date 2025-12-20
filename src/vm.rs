use crate::grammar::{AST, Instruction, Operator, StructFieldInit, Type, StructInstance};
use std::collections::{HashMap, HashSet};

pub struct VM {
    stack: Vec<Type>,
    environment: HashMap<String, Type>,
    pointer: usize,
    immutable_stack: Vec<HashMap<String, Type>>,
    code: Vec<Instruction>,
    labels: HashMap<String, usize>,
    struct_defs: HashMap<String, Vec<(String, Option<StructFieldInit>)>>,
    heap: Vec<StructInstance>,
}

impl VM {
    pub fn new(code: Vec<Instruction>) -> Self {
        let labels = Self::build_labels(&code);
        Self {
            stack: Vec::new(),
            environment: HashMap::new(),
            immutable_stack: vec![HashMap::new()],
            pointer: 0,
            code,
            labels,
            struct_defs: HashMap::new(),
            heap: Vec::new(),
        }
    }

    fn build_labels(code: &Vec<Instruction>) -> HashMap<String, usize> {
        let mut labels = HashMap::new();
        for (i, instr) in code.iter().enumerate() {
            if let Instruction::Label(name) = instr {
                labels.insert(name.clone(), i);
            }
        }
        labels
    }

    pub fn run(&mut self) {
        while self.pointer < self.code.len() {
            let instr = self.code[self.pointer].clone();

            match instr {
                Instruction::Push(n) => self.stack.push(Type::Integer(n)),

                Instruction::Load(name) => {
                    let v = self
                        .find_immutable(&name)
                        .cloned()
                        .or_else(|| self.environment.get(&name).cloned())
                        .unwrap();
                    let out = self.load_value(v);
                    self.stack.push(out);
                }

                Instruction::Store(name) => {
                    if self.immutable_exists(&name) {
                        panic!()
                    }
                    let v = self.pop();
                    self.environment.insert(name, v);
                }

                Instruction::StoreReactive(name, ast) => {
                    if self.immutable_exists(&name) {
                        panic!()
                    }
                    let frozen = self.freeze_ast(ast);
                    self.environment.insert(name, Type::LazyInteger(frozen));
                }

                Instruction::StoreImmutable(name) => {
                    let v = self.pop();
                    let scope = self.immutable_stack.last_mut().unwrap();
                    if scope.contains_key(&name) {
                        panic!()
                    }
                    scope.insert(name, v);
                }

                Instruction::Add => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer(b + a));
                }

                Instruction::Sub => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer(b - a));
                }

                Instruction::Mul => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer(b * a));
                }

                Instruction::Div => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer(b / a));
                }

                Instruction::Greater => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer((b > a) as i32));
                }

                Instruction::Less => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer((b < a) as i32));
                }

                Instruction::Equal => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer((b == a) as i32));
                }

                Instruction::NotEqual => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer((b != a) as i32));
                }

                Instruction::GreaterEqual => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer((b >= a) as i32));
                }

                Instruction::LessEqual => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer((b <= a) as i32));
                }

                Instruction::And => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer(((b > 0) && (a > 0)) as i32));
                }

                Instruction::Or => {
                    let a = self.pop_int();
                    let b = self.pop_int();
                    self.stack.push(Type::Integer(((b > 0) || (a > 0)) as i32));
                }

                Instruction::Print => {
                    let v = self.pop();
                    let n = self.as_int(v);
                    print!("{n}");
                }

                Instruction::Println => {
                    let v = self.pop();
                    let n = self.as_int(v);
                    println!("{n}");
                }

                Instruction::ArrayNew => {
                    let n = {
                        let v = self.pop();
                        self.as_int(v)
                    };
                    self.stack
                        .push(Type::Array(vec![Type::Integer(0); n as usize]));
                }

                Instruction::ArrayGet => {
                    let idx = {
                        let v = self.pop();
                        self.as_int(v) as usize
                    };

                    let arr = self.pop();

                    match arr {
                        Type::Array(v) => {
                            let elem = v.get(idx).cloned().unwrap();
                            let out = self.load_value(elem);
                            self.stack.push(out);
                        }
                        _ => panic!(),
                    }
                }

                Instruction::StoreIndex(name) => {
                    if self.immutable_exists(&name) {
                        panic!()
                    }
                    let val = self.pop();
                    let idx = {
                        let v = self.pop();
                        self.as_int(v) as usize
                    };
                    let entry = self.environment.get_mut(&name).unwrap();
                    match entry {
                        Type::Array(v) => {
                            v[idx] = val;
                        }
                        _ => panic!(),
                    }
                }

                Instruction::StoreIndexReactive(name, ast) => {
                    if self.immutable_exists(&name) {
                        panic!()
                    }
                    let idx = {
                        let v = self.pop();
                        self.as_int(v) as usize
                    };
                    let frozen = self.freeze_ast(ast);
                    let entry = self.environment.get_mut(&name).unwrap();
                    match entry {
                        Type::Array(v) => {
                            v[idx] = Type::LazyInteger(frozen);
                        }
                        _ => panic!(),
                    }
                }

                Instruction::StoreFunction(name, params, body) => {
                    self.environment.insert(name, Type::Function { params, body });
                }

                Instruction::Call(name, argc) => {
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        args.push(self.pop());
                    }
                    args.reverse();
                    let f = self.environment.get(&name).cloned().unwrap();
                    let ret = self.call_function(f, args);
                    self.stack.push(ret);
                }

                Instruction::StoreStruct(name, fields) => {
                    self.struct_defs.insert(name, fields);
                }

                Instruction::NewStruct(name) => {
                    let def = self.struct_defs.get(&name).cloned().unwrap();
                    let inst = self.instantiate_struct(def);
                    self.stack.push(inst);
                }

                Instruction::FieldGet(field) => {
                    let obj = self.pop();
                    match obj {
                        Type::StructRef(id) => {
                            let v = self.heap[id].fields.get(&field).cloned().unwrap();
                            let out = match v {
                                Type::LazyInteger(ast) => {
                                    self.eval_reactive_field(id, *ast)
                                }
                                other => other,
                            };
                            self.stack.push(out);
                        }
                        _ => panic!(),
                    }
                }


                Instruction::FieldSet(field) => {
                    let val = self.pop();
                    let obj = self.pop();
                    match obj {
                        Type::StructRef(id) => {
                            if self.heap[id].immutables.contains(&field) {
                                panic!()
                            }
                            let stored = match val {
                                Type::LazyInteger(ast) => Type::Integer(self.evaluate(*ast)),
                                other => other,
                            };

                            self.heap[id].fields.insert(field, stored);

                        }
                        _ => panic!(),
                    }
                }

                Instruction::FieldSetReactive(field, ast) => {
                    let obj = self.pop();
                    match obj {
                        Type::StructRef(id) => {
                            if self.heap[id].immutables.contains(&field) {
                                panic!()
                            }
                            let frozen = self.freeze_ast(ast);
                            self.heap[id].fields.insert(field, Type::LazyInteger(frozen));
                        }
                        _ => panic!(),
                    }
                }

                Instruction::PushImmutableContext => {
                    self.immutable_stack.push(HashMap::new());
                }

                Instruction::PopImmutableContext => {
                    if self.immutable_stack.len() <= 1 {
                        panic!()
                    }
                    self.immutable_stack.pop();
                }

                Instruction::ClearImmutableContext => {
                    self.immutable_stack.last_mut().unwrap().clear();
                }

                Instruction::Label(_) => {}

                Instruction::Jump(l) => {
                    self.pointer = self.labels[&l];
                    continue;
                }

                Instruction::JumpIfZero(l) => {
                    let v = self.pop();
                    let n = self.as_int(v);
                    if n == 0 {
                        self.pointer = self.labels[&l];
                        continue;
                    }
                }
            }

            self.pointer += 1;
        }
    }

    fn instantiate_struct(&mut self, fields: Vec<(String, Option<StructFieldInit>)>) -> Type {
        let mut map = HashMap::new();
        let mut imm = HashSet::new();

        for (name, init) in fields {
            match init {
                None => {
                    map.insert(name, Type::Integer(0));
                }
                Some(StructFieldInit::Mutable(ast)) => {
                    let n = self.evaluate(ast);
                    map.insert(name, Type::Integer(n));
                }
                Some(StructFieldInit::Immutable(ast)) => {
                    let n = self.evaluate(ast);
                    imm.insert(name.clone());
                    map.insert(name, Type::Integer(n));
                }
                Some(StructFieldInit::Reactive(ast)) => {
                    let frozen = self.freeze_ast(Box::new(ast));
                    map.insert(name, Type::LazyInteger(frozen));
                }
            }
        }

        let id = self.heap.len();
        self.heap.push(StructInstance { fields: map, immutables: imm });
        Type::StructRef(id)
    }

    fn call_function(&mut self, f: Type, args: Vec<Type>) -> Type {
        match f {
            Type::Function { params, body } => {
                self.immutable_stack.push(HashMap::new());
                {
                    let scope = self.immutable_stack.last_mut().unwrap();
                    for (p, v) in params.into_iter().zip(args) {
                        scope.insert(p, v);
                    }
                }

                let mut ret = Type::Integer(0);
                for stmt in body {
                    match stmt {
                        AST::Return(expr) => {
                            ret = match expr {
                                Some(e) => self.eval_value(*e),   
                                None => Type::Integer(0),
                            };
                            break;
                        }
                        other => self.execute_ast(other),
                    }
                }

                self.immutable_stack.pop();
                ret
            }
            _ => panic!(),
        }
    }

fn execute_ast(&mut self, ast: AST) {
    let stack_len = self.stack.len(); 

    let mut tmp = Vec::new();
    let mut lg = crate::compiler::LabelGenerator::new();
    let mut bs = Vec::new();
    crate::compiler::compile(ast, &mut tmp, &mut lg, &mut bs);

    let saved_code = std::mem::replace(&mut self.code, tmp);
    let saved_labels = std::mem::replace(&mut self.labels, Self::build_labels(&self.code));
    let saved_ptr = self.pointer;
    self.pointer = 0;

    self.run();

    self.stack.truncate(stack_len);

    self.code = saved_code;
    self.labels = saved_labels;
    self.pointer = saved_ptr;
}


    fn evaluate(&mut self, ast: AST) -> i32 {
        match ast {
            AST::Number(n) => n,

            AST::Var(name) => {
                let v = self
                    .find_immutable(&name)
                    .cloned()
                    .or_else(|| self.environment.get(&name).cloned())
                    .unwrap();
                self.as_int(v)
            }

            AST::Operation(l, op, r) => {
                let a = self.evaluate(*l);
                let b = self.evaluate(*r);
                match op {
                    Operator::Addition => a + b,
                    Operator::Subtraction => a - b,
                    Operator::Multiplication => a * b,
                    Operator::Division => a / b,
                    Operator::Greater => (a > b) as i32,
                    Operator::Less => (a < b) as i32,
                    Operator::Equal => (a == b) as i32,
                    Operator::NotEqual => (a != b) as i32,
                    Operator::GreaterEqual => (a >= b) as i32,
                    Operator::LessEqual => (a <= b) as i32,
                    Operator::And => ((a > 0) && (b > 0)) as i32,
                    Operator::Or => ((a > 0) || (b > 0)) as i32,
                }
            }

            AST::Index(base, index) => {
                let idx = self.evaluate(*index) as usize;
                let arr = match *base {
                    AST::Var(name) => self
                        .find_immutable(&name)
                        .cloned()
                        .or_else(|| self.environment.get(&name).cloned())
                        .unwrap(),
                    _ => panic!(),
                };
                match arr {
                    Type::Array(v) => self.as_int(v.get(idx).cloned().unwrap()),
                    _ => panic!(),
                }
            }

            AST::FieldAccess(base, field) => {
                let obj = self.eval_value(*base);
                match obj {
                    Type::StructRef(id) => {
                        let v = self.heap[id].fields.get(&field).cloned().unwrap();
                        match v {
                            Type::LazyInteger(ast) => {
                                if let Type::Integer(n) = self.eval_reactive_field(id, *ast) {
                                    n
                                } else {
                                    unreachable!()
                                }
                            }
                            other => self.as_int(other),
                        }
                    }
                    _ => panic!(),
                }
            }


            AST::Call { name, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_value(a)); 
                }
                let f = self.environment.get(&name).cloned().unwrap();
                let out = self.call_function(f, vals);
                self.as_int(out) 
            }

            _ => panic!(),
        }
    }
fn eval_reactive_field(&mut self, id: usize, ast: AST) -> Type {
    self.immutable_stack.push(HashMap::new());

    {
        let scope = self.immutable_stack.last_mut().unwrap();
        for (k, v) in self.heap[id].fields.iter() {
            scope.insert(k.clone(), v.clone());
        }
    }

    let result = Type::Integer(self.evaluate(ast));

    self.immutable_stack.pop();
    result
}

fn eval_value(&mut self, ast: AST) -> Type {
    match ast {
        AST::Number(n) => Type::Integer(n),

        AST::Var(name) => {
            self.find_immutable(&name)
                .cloned()
                .or_else(|| self.environment.get(&name).cloned())
                .unwrap()
        }

AST::FieldAccess(base, field) => {
    let obj = self.eval_value(*base);
    match obj {
        Type::StructRef(id) => {
            let v = self.heap[id].fields.get(&field).cloned().unwrap();
            match v {
                Type::LazyInteger(ast) => self.eval_reactive_field(id, *ast),
                other => other,
            }
        }
        _ => panic!(),
    }
}

        AST::Index(base, index) => {
            let idx = self.evaluate(*index) as usize;
            let arr = self.eval_value(*base);
            match arr {
                Type::Array(v) => v[idx].clone(),
                _ => panic!(),
            }
        }

        AST::Call { name, args } => {
            let mut vals = Vec::new();
            for a in args {
                vals.push(Type::Integer(self.evaluate(a)));
            }
            let f = self.environment.get(&name).cloned().unwrap();
            self.call_function(f, vals)
        }

        AST::Operation(_, _, _) => Type::Integer(self.evaluate(ast)),

        _ => panic!(),
    }
}


    fn as_int(&mut self, v: Type) -> i32 {
        match v {
            Type::Integer(n) => n,
            Type::LazyInteger(ast) => self.evaluate(*ast),
            Type::Array(v) => v.len() as i32,
            Type::StructRef(_) => panic!(),
            Type::Function { .. } => panic!(),
        }
    }

    fn load_value(&mut self, v: Type) -> Type {
        match v {
            Type::LazyInteger(ast) => Type::Integer(self.evaluate(*ast)),
            other => other,
        }
    }

    fn freeze_ast(&self, ast: Box<AST>) -> Box<AST> {
        match *ast {
            AST::Var(name) => {
                if let Some(Type::Integer(n)) = self.find_immutable(&name) {
                    Box::new(AST::Number(*n))
                } else {
                    Box::new(AST::Var(name))
                }
            }
            AST::Number(n) => Box::new(AST::Number(n)),
            AST::Operation(l, o, r) => Box::new(AST::Operation(self.freeze_ast(l), o, self.freeze_ast(r))),
            AST::Index(b, i) => Box::new(AST::Index(self.freeze_ast(b), self.freeze_ast(i))),
            AST::FieldAccess(b, f) => Box::new(AST::FieldAccess(self.freeze_ast(b), f)),
            other => Box::new(other),
        }
    }

    fn find_immutable(&self, name: &str) -> Option<&Type> {
        self.immutable_stack.iter().rev().find_map(|s| s.get(name))
    }

    fn immutable_exists(&self, name: &str) -> bool {
        self.find_immutable(name).is_some()
    }

    fn pop(&mut self) -> Type {
        self.stack.pop().unwrap()
    }

    fn pop_int(&mut self) -> i32 {
        let v = self.pop();
        self.as_int(v)
    }
}
