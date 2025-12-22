use crate::grammar::{AST, Instruction, LValue, Operator, StructFieldInit, StructInstance, Type};
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
    array_heap: Vec<Vec<Type>>,
    imported_modules: HashSet<String>,
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
            array_heap: Vec::new(),
            imported_modules: HashSet::new(),
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
                        .unwrap_or_else(|| panic!("undefined variable: {name}"));

                    let out = self.load_value(v);
                    self.stack.push(out);
                }
                Instruction::Store(name) => {
                    if self.immutable_exists(&name) {
                        panic!("cannot assign to immutable variable `{name}`");
                    }

                    let v = self.pop();
                    self.environment.insert(name, v);
                }
                Instruction::Import(path) => {
                    let module_name = path.join(".");

                    if self.imported_modules.contains(&module_name) {
                    } else {
                        self.imported_modules.insert(module_name.clone());
                        self.import_module(path);
                    }
                }
                Instruction::StoreReactive(name, ast) => {
                    if self.immutable_exists(&name) {
                        panic!("cannot reactively assign to immutable variable `{name}`");
                    }
                    let frozen = self.freeze_ast(ast);
                    self.environment.insert(name, Type::LazyValue(frozen));
                }
                Instruction::StoreImmutable(name) => {
                    let v = self.pop();
                    let scope = self.immutable_stack.last_mut().unwrap();
                    if scope.contains_key(&name) {
                        panic!("cannot reassign immutable variable `{name}`");
                    }
                    scope.insert(name, v);
                }
                Instruction::Add => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    match (lhs, rhs) {
                        (Type::Char(c), Type::Integer(n)) | (Type::Integer(n), Type::Char(c)) => {
                            self.stack.push(Type::Char((c as i32 + n) as u32));
                        }

                        (Type::Char(a), Type::Char(b)) => {
                            self.stack.push(Type::Char((a + b) as u32));
                        }

                        (a, b) => {
                            let ai = self.as_int(a);
                            let bi = self.as_int(b);
                            self.stack.push(Type::Integer(ai + bi));
                        }
                    }
                }

                Instruction::Sub => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    match (lhs, rhs) {
                        (Type::Char(c), Type::Integer(n)) | (Type::Integer(n), Type::Char(c)) => {
                            self.stack.push(Type::Char((c as i32 - n) as u32));
                        }

                        (Type::Char(a), Type::Char(b)) => {
                            self.stack.push(Type::Char((a - b) as u32));
                        }

                        (a, b) => {
                            let ai = self.as_int(a);
                            let bi = self.as_int(b);
                            self.stack.push(Type::Integer(ai - bi));
                        }
                    }
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
                    match v {
                        Type::Char(c) => {
                            print!("{}", char::from_u32(c).unwrap());
                        }

                        Type::Integer(n) => {
                            print!("{n}");
                        }

                        Type::ArrayRef(id) => {
                            let elems = self.array_heap[id].clone();

                            for elem in elems {
                                let v = self.load_value(elem);
                                match v {
                                    Type::Char(c) => print!("{}", char::from_u32(c).unwrap()),
                                    _ => panic!("non-char in string"),
                                }
                            }
                        }

                        _ => panic!("cannot print value"),
                    }
                }

                Instruction::Println => {
                    let v = self.pop();
                    match v {
                        Type::Char(c) => {
                            print!("{}", char::from_u32(c).unwrap());
                        }

                        Type::Integer(n) => {
                            print!("{n}");
                        }

                        Type::ArrayRef(id) => {
                            let elems = self.array_heap[id].clone();

                            for elem in elems {
                                let v = self.load_value(elem);
                                match v {
                                    Type::Char(c) => print!("{}", char::from_u32(c).unwrap()),
                                    _ => panic!("non-char in string"),
                                }
                            }
                        }

                        _ => panic!("cannot print value"),
                    }
                    println!();
                }

                Instruction::ArrayNew => {
                    let n = {
                        let v = self.pop();
                        self.as_int(v)
                    };
                    let id = self.array_heap.len();
                    self.array_heap.push(vec![Type::Integer(0); n as usize]);
                    self.stack.push(Type::ArrayRef(id));
                }
                Instruction::ArrayGet => {
                    let idx = {
                        let v = self.pop();
                        let i = self.as_int(v);
                        if i < 0 {
                            panic!("array index out of bounds: index {} is negative", i);
                        }
                        i as usize
                    };

                    let arr = self.pop();

                    match arr {
                        Type::ArrayRef(id) => {
                            let arr_ref = &self.array_heap[id];
                            let len = arr_ref.len();

                            if idx >= len {
                                panic!("array index out of bounds: index {}, length {}", idx, len);
                            }

                            let elem = arr_ref[idx].clone();
                            let out = self.load_value(elem);
                            self.stack.push(out);
                        }
                        other => {
                            panic!("type error: attempted to index non-array value {:?}", other);
                        }
                    }
                }
                Instruction::StoreIndex(name) => {
                    let val = self.pop();
                    let idx = {
                        let v = self.pop();
                        self.as_int(v) as usize
                    };

                    let target = self
                        .find_immutable(&name)
                        .cloned()
                        .or_else(|| self.environment.get(&name).cloned())
                        .unwrap();

                    match target {
                        Type::ArrayRef(id) => {
                            self.array_heap[id][idx] = val;
                        }
                        _ => panic!(),
                    }
                }
                Instruction::StoreIndexReactive(name, ast) => {
                    let idx = {
                        let v = self.pop();
                        self.as_int(v) as usize
                    };

                    let frozen = self.freeze_ast(ast);

                    let target = self
                        .find_immutable(&name)
                        .cloned()
                        .or_else(|| self.environment.get(&name).cloned())
                        .unwrap();

                    match target {
                        Type::ArrayRef(id) => {
                            self.array_heap[id][idx] = Type::LazyValue(frozen);
                        }
                        _ => panic!(),
                    }
                }
                Instruction::StoreFunction(name, params, body) => {
                    self.environment
                        .insert(name, Type::Function { params, body });
                }
                Instruction::Call(name, argc) => {
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        args.push(self.pop());
                    }
                    args.reverse();

                    let f = match self.environment.get(&name) {
                        Some(v) => v.clone(),
                        None => {
                            panic!(
                                "call error: `{}` is not defined (attempted to call with {} argument(s))",
                                name, argc
                            )
                        }
                    };

                    match f {
                        Type::Function { .. } => {
                            let ret = self.call_function(f, args);
                            self.stack.push(ret);
                        }
                        other => {
                            panic!(
                                "call error: `{}` is not a function (found {:?})",
                                name, other
                            )
                        }
                    }
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
                                Type::LazyValue(ast) => self.eval_reactive_field(*ast),
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
                                Type::LazyValue(ast) => self.eval_value(*ast),
                                other => other,
                            };

                            self.heap[id].fields.insert(field, stored);
                        }
                        _ => panic!(),
                    }
                }
                Instruction::Return => {
                    return;
                }
                Instruction::FieldSetReactive(field, ast) => {
                    let obj = self.pop();
                    match obj {
                        Type::StructRef(id) => {
                            if self.heap[id].immutables.contains(&field) {
                                panic!()
                            }
                            let frozen = self.freeze_ast(ast);
                            self.heap[id].fields.insert(field, Type::LazyValue(frozen));
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
                Instruction::ArrayLValue => {
                    let idx = {
                        let v = self.pop();
                        self.as_int(v) as usize
                    };

                    let base = self.pop();

                    match base {
                        Type::ArrayRef(id) => {
                            self.stack.push(Type::LValue(LValue::ArrayElem {
                                array_id: id,
                                index: idx,
                            }));
                        }

                        Type::LValue(LValue::ArrayElem { array_id, index }) => {
                            let nested = match &self.array_heap[array_id][index] {
                                Type::ArrayRef(id) => *id,
                                _ => panic!("indexing non-array"),
                            };

                            self.stack.push(Type::LValue(LValue::ArrayElem {
                                array_id: nested,
                                index: idx,
                            }));
                        }

                        Type::LValue(LValue::StructField { struct_id, field }) => {
                            let arr = self.heap[struct_id]
                                .fields
                                .get(&field)
                                .expect("missing field");

                            let array_id = match arr {
                                Type::ArrayRef(id) => *id,
                                _ => panic!("indexing non-array struct field"),
                            };

                            self.stack.push(Type::LValue(LValue::ArrayElem {
                                array_id,
                                index: idx,
                            }));
                        }

                        _ => panic!("invalid ArrayLValue base"),
                    }
                }
                Instruction::FieldLValue(field) => {
                    let base = self.pop();

                    match base {
                        Type::StructRef(id) => {
                            self.stack.push(Type::LValue(LValue::StructField {
                                struct_id: id,
                                field,
                            }));
                        }

                        Type::LValue(LValue::ArrayElem { array_id, index }) => {
                            let elem = &self.array_heap[array_id][index];
                            match elem {
                                Type::StructRef(id) => {
                                    self.stack.push(Type::LValue(LValue::StructField {
                                        struct_id: *id,
                                        field,
                                    }));
                                }
                                _ => panic!("FieldLValue on non-struct array element"),
                            }
                        }

                        _ => panic!("invalid FieldLValue base"),
                    }
                }
                Instruction::StoreThrough => {
                    let value = self.pop();
                    let target = self.pop();

                    match target {
                        Type::LValue(LValue::ArrayElem { array_id, index }) => {
                            if index >= self.array_heap[array_id].len() {
                                panic!(
                                    "array assignment out of bounds: index {} but length {}",
                                    index,
                                    self.array_heap[array_id].len()
                                );
                            }

                            self.array_heap[array_id][index] = value;
                        }

                        Type::LValue(LValue::StructField { struct_id, field }) => {
                            if self.heap[struct_id].immutables.contains(&field) {
                                panic!("cannot assign to immutable field '{}'", field);
                            }

                            self.heap[struct_id].fields.insert(field, value);
                        }

                        other => {
                            panic!(
                                "internal error: StoreThrough target is not an lvalue (got {:?})",
                                other
                            );
                        }
                    }
                }
                Instruction::StoreThroughReactive(ast) => {
                    let target = self.pop();
                    let frozen = self.freeze_ast(ast);

                    match target {
                        Type::LValue(LValue::ArrayElem { array_id, index }) => {
                            self.array_heap[array_id][index] = Type::LazyValue(frozen);
                        }

                        Type::LValue(LValue::StructField { struct_id, field }) => {
                            if self.heap[struct_id].immutables.contains(&field) {
                                panic!("immutable field");
                            }
                            self.heap[struct_id]
                                .fields
                                .insert(field, Type::LazyValue(frozen));
                        }

                        _ => panic!("StoreThroughReactive target is not an lvalue"),
                    }
                }
                Instruction::Modulo => {
                    let rhs = self.pop();
                    let lhs = self.pop();

                    match (lhs, rhs) {
                        (Type::Char(c), Type::Integer(n)) | (Type::Integer(n), Type::Char(c)) => {
                            self.stack.push(Type::Char((c as i32 % n) as u32));
                        }

                        (Type::Char(a), Type::Char(b)) => {
                            self.stack.push(Type::Char((a % b) as u32));
                        }

                        (a, b) => {
                            let ai = self.as_int(a);
                            let bi = self.as_int(b);
                            self.stack.push(Type::Integer(ai % bi));
                        }
                    }
                }
                Instruction::PushChar(c) => self.stack.push(Type::Char(c)),
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
                    let v = self.eval_value(ast);
                    map.insert(name, self.clone_value(v));
                }

                Some(StructFieldInit::Immutable(ast)) => {
                    let v = self.eval_value(ast);
                    imm.insert(name.clone());
                    map.insert(name, self.clone_value(v));
                }

                Some(StructFieldInit::Reactive(ast)) => {
                    let frozen = self.freeze_ast(Box::new(ast));
                    map.insert(name, Type::LazyValue(frozen));
                }
            }
        }

        let id = self.heap.len();
        self.heap.push(StructInstance {
            fields: map,
            immutables: imm,
        });

        Type::StructRef(id)
    }

    fn clone_value(&mut self, v: Type) -> Type {
        match v {
            Type::ArrayRef(id) => {
                let new_id = self.array_heap.len();
                self.array_heap.push(self.array_heap[id].clone());
                Type::ArrayRef(new_id)
            }
            Type::StructRef(id) => {
                let inst = self.heap[id].clone();
                let new_id = self.heap.len();
                self.heap.push(inst);
                Type::StructRef(new_id)
            }
            Type::LazyValue(ast) => Type::LazyValue(ast),
            Type::Integer(n) => Type::Integer(n),
            Type::Function { params, body } => Type::Function { params, body },
            Type::LValue(_) => panic!("cannot clone lvalue"),
            Type::Char(c) => Type::Char(c),
        }
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

                let mut code = Vec::new();
                let mut lg = crate::compiler::LabelGenerator::new();
                let mut bs = Vec::new();

                crate::compiler::compile(AST::Program(body), &mut code, &mut lg, &mut bs);

                let saved_code = std::mem::replace(&mut self.code, code);
                let saved_labels =
                    std::mem::replace(&mut self.labels, Self::build_labels(&self.code));
                let saved_ptr = self.pointer;
                let saved_stack_len = self.stack.len();

                self.pointer = 0;
                self.run();

                let ret = if self.stack.len() > saved_stack_len {
                    self.pop()
                } else {
                    Type::Integer(0)
                };

                self.code = saved_code;
                self.labels = saved_labels;
                self.pointer = saved_ptr;
                self.immutable_stack.pop();

                ret
            }
            _ => panic!("attempted to call non-function"),
        }
    }

    fn evaluate(&mut self, ast: AST) -> i32 {
        match ast {
            AST::Number(n) => n,
            AST::Char(c) => c as i32,
            AST::Var(name) => {
                let v = self
                    .find_immutable(&name)
                    .cloned()
                    .or_else(|| self.environment.get(&name).cloned())
                    .unwrap();
                self.as_int(v)
            }
            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let c = self.evaluate(*cond);
                if c != 0 {
                    self.evaluate(*then_expr)
                } else {
                    self.evaluate(*else_expr)
                }
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
                    Operator::Modulo => a % b as i32,
                }
            }

            AST::Index(base, index) => {
                let idx = self.evaluate(*index) as usize;
                let arr = self.eval_value(*base);

                match arr {
                    Type::ArrayRef(id) => {
                        let elem = self.array_heap[id].get(idx).cloned().unwrap();
                        self.as_int(elem)
                    }
                    _ => panic!(),
                }
            }

            AST::FieldAccess(base, field) => {
                let obj = self.eval_value(*base);
                match obj {
                    Type::StructRef(id) => {
                        let v = self.heap[id].fields.get(&field).cloned().unwrap();
                        match v {
                            Type::LazyValue(ast) => {
                                if let Type::Integer(n) = self.eval_reactive_field(*ast) {
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

            other => {
                println!("evaluate(): unsupported AST: {other:?}");
                panic!()
            }
        }
    }
    fn eval_reactive_field(&mut self, ast: AST) -> Type {
        let saved_stack = std::mem::take(&mut self.immutable_stack);
        self.immutable_stack.push(HashMap::new());
        let result = self.eval_value(ast);
        self.immutable_stack = saved_stack;

        result
    }

    fn eval_value(&mut self, ast: AST) -> Type {
        match ast {
            AST::Number(n) => Type::Integer(n),
            AST::Char(c) => Type::Char(c),
            AST::Var(name) => self
                .find_immutable(&name)
                .cloned()
                .or_else(|| self.environment.get(&name).cloned())
                .unwrap(),
            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let c = self.evaluate(*cond);
                if c != 0 {
                    self.eval_value(*then_expr)
                } else {
                    self.eval_value(*else_expr)
                }
            }

            AST::ArrayNew(size_ast) => {
                let n = self.evaluate(*size_ast) as usize;
                let id = self.array_heap.len();
                self.array_heap.push(vec![Type::Integer(0); n]);
                Type::ArrayRef(id)
            }

            AST::FieldAccess(base, field) => {
                let obj = self.eval_value(*base);
                match obj {
                    Type::StructRef(id) => {
                        let v = self.heap[id].fields.get(&field).cloned().unwrap();
                        match v {
                            Type::LazyValue(ast) => self.eval_reactive_field(*ast),
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
                    Type::ArrayRef(id) => {
                        let elem = self.array_heap[id].get(idx).cloned().unwrap();
                        self.load_value(elem)
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
                self.call_function(f, vals)
            }

            AST::Operation(l, op, r) => {
                let lv = self.eval_value((*l).clone());
                let rv = self.eval_value((*r).clone());

                match (lv, rv, &op) {
                    (Type::Char(c), Type::Integer(n), Operator::Addition) => {
                        Type::Char((c as i32 + n) as u32)
                    }
                    (Type::Integer(n), Type::Char(c), Operator::Addition) => {
                        Type::Char((c as i32 + n) as u32)
                    }
                    _ => Type::Integer(self.evaluate(AST::Operation(l, op, r))),
                }
            }

            _ => panic!(),
        }
    }

    fn as_int(&mut self, v: Type) -> i32 {
        match v {
            Type::Integer(n) => n,
            Type::Char(c) => c as i32,
            Type::LazyValue(ast) => self.evaluate(*ast),
            Type::ArrayRef(id) => self.array_heap[id].len() as i32,
            Type::StructRef(_) => panic!("cannot coerce struct ref to int"),
            Type::Function { .. } => panic!("cannot coerce function to int"),
            Type::LValue(_) => panic!("cannot coerce lvalue to int"),
        }
    }

    fn load_value(&mut self, v: Type) -> Type {
        match v {
            Type::LazyValue(ast) => self.eval_value(*ast),
            Type::LValue(_) => panic!("cannot load lvalue directly"),
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
            AST::Char(c) => Box::new(AST::Char(c)),
            AST::Operation(l, o, r) => {
                Box::new(AST::Operation(self.freeze_ast(l), o, self.freeze_ast(r)))
            }
            AST::Index(b, i) => Box::new(AST::Index(self.freeze_ast(b), self.freeze_ast(i))),
            AST::FieldAccess(b, f) => Box::new(AST::FieldAccess(self.freeze_ast(b), f)),
            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => Box::new(AST::Ternary {
                cond: self.freeze_ast(cond),
                then_expr: self.freeze_ast(then_expr),
                else_expr: self.freeze_ast(else_expr),
            }),

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
    fn import_module(&mut self, path: Vec<String>) {
        let file_path = format!("project/{}.rx", path.join("/"));

        let source = std::fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("could not import module `{}`", file_path));

        let tokens = crate::tokenizer::tokenize(&source);
        //println!("TOKENS FROM {}: {:?}", file_path, tokens);
        let ast = crate::parser::parse(tokens);

        let mut code = Vec::new();
        let mut lg = crate::compiler::LabelGenerator::new();
        let mut bs = Vec::new();

        crate::compiler::compile(ast, &mut code, &mut lg, &mut bs);

        let saved_code = std::mem::replace(&mut self.code, code);
        let saved_labels = std::mem::replace(&mut self.labels, Self::build_labels(&self.code));
        let saved_ptr = self.pointer;

        self.pointer = 0;
        self.run();

        self.code = saved_code;
        self.labels = saved_labels;
        self.pointer = saved_ptr;
    }
}
