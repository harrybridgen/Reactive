use crate::grammar::{AST, Instruction, LValue, Operator, StructFieldInit, StructInstance, Type};
use std::collections::{HashMap, HashSet};

pub struct VM {
    // Operand stack
    stack: Vec<Type>,

    // Global mutable environment (top-level only)
    global_env: HashMap<String, Type>,

    // Local mutable environment (function scope)
    local_env: Option<HashMap<String, Type>>,

    // Immutable scopes (:= bindings, function parameters, and temporary reactive contexts)
    immutable_stack: Vec<HashMap<String, Type>>,

    // Bytecode and dispatch state
    pointer: usize,
    code: Vec<Instruction>,
    labels: HashMap<String, usize>,

    // Runtime type heaps
    struct_defs: HashMap<String, Vec<(String, Option<StructFieldInit>)>>,
    heap: Vec<StructInstance>,
    array_heap: Vec<Vec<Type>>,

    // Module import memoization
    imported_modules: HashSet<String>,

    // Debugging
    debug: bool,
    debug_reactive_ctx: Vec<String>,
}

impl VM {
    pub fn new(code: Vec<Instruction>) -> Self {
        let labels = Self::build_labels(&code);
        Self {
            stack: Vec::new(),
            global_env: HashMap::new(),
            local_env: None,
            immutable_stack: vec![HashMap::new()],
            pointer: 0,
            code,
            labels,
            struct_defs: HashMap::new(),
            heap: Vec::new(),
            array_heap: Vec::new(),
            imported_modules: HashSet::new(),
            debug: true,
            debug_reactive_ctx: Vec::new(),
        }
    }

    fn build_labels(code: &[Instruction]) -> HashMap<String, usize> {
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
            // Clone because instructions store owned Strings/AST in some variants.
            let instr = self.code[self.pointer].clone();

            match instr {
                Instruction::Push(n) => self.stack.push(Type::Integer(n)),
                Instruction::PushChar(c) => self.stack.push(Type::Char(c)),
                Instruction::Load(name) => {
                    let v = self
                        .lookup_var(&name)
                        .cloned()
                        .unwrap_or_else(|| panic!("undefined variable: {name}"));

                    let value = self.force(v);
                    self.stack.push(value);
                }

                Instruction::Store(name) => {
                    self.ensure_mutable_binding(&name);
                    let v = self.pop();
                    match &mut self.local_env {
                        Some(env) => {
                            env.insert(name, v);
                        }
                        None => {
                            self.global_env.insert(name, v);
                        }
                    }
                }
                Instruction::StoreImmutable(name) => {
                    let v = self.pop();
                    let scope = self
                        .immutable_stack
                        .last_mut()
                        .expect("internal error: no immutable scope");
                    if scope.contains_key(&name) {
                        panic!("cannot reassign immutable variable `{name}`");
                    }
                    scope.insert(name, v);
                }
                Instruction::StoreReactive(name, ast) => {
                    self.ensure_mutable_binding(&name);
                    let frozen = self.freeze_ast(ast);
                    let captured = self.capture_immutables_for_ast(&frozen);
                    match &mut self.local_env {
                        Some(env) => {
                            env.insert(name, Type::LazyValue(frozen, captured));
                        }
                        None => {
                            self.global_env
                                .insert(name, Type::LazyValue(frozen, captured));
                        }
                    }
                }
                Instruction::Import(path) => {
                    let module_name = path.join(".");
                    if !self.imported_modules.contains(&module_name) {
                        self.imported_modules.insert(module_name.clone());
                        self.import_module(path);
                    }
                }

                // ----- Arithmetic / Logic -----
                Instruction::Add => self.exec_add(),
                Instruction::Sub => self.exec_sub(),
                Instruction::Mul => self.exec_mul(),
                Instruction::Div => self.exec_div(),
                Instruction::Modulo => self.exec_modulo(),
                Instruction::Greater => self.exec_cmp(|b, a| (b > a) as i32),
                Instruction::Less => self.exec_cmp(|b, a| (b < a) as i32),
                Instruction::Equal => self.exec_cmp(|b, a| (b == a) as i32),
                Instruction::NotEqual => self.exec_cmp(|b, a| (b != a) as i32),
                Instruction::GreaterEqual => self.exec_cmp(|b, a| (b >= a) as i32),
                Instruction::LessEqual => self.exec_cmp(|b, a| (b <= a) as i32),
                Instruction::And => self.exec_cmp(|b, a| ((b > 0) && (a > 0)) as i32),
                Instruction::Or => self.exec_cmp(|b, a| ((b > 0) || (a > 0)) as i32),

                // ----- Printing -----
                Instruction::Print => {
                    let v = self.pop();
                    self.print_value(v, false);
                }
                Instruction::Println => {
                    let v = self.pop();
                    self.print_value(v, true);
                }

                // ----- Arrays -----
                Instruction::ArrayNew => {
                    let size_val = self.pop();
                    let n = self.as_usize_nonneg(size_val, "array size");

                    let id = self.array_heap.len();
                    self.array_heap.push(vec![Type::Integer(0); n]);
                    self.stack.push(Type::ArrayRef(id));
                }
                Instruction::ArrayGet => {
                    // pop index
                    let idx_val = self.pop();
                    let idx = self.as_usize_nonneg(idx_val, "array index");

                    // pop array
                    let arr_val = self.pop();
                    let arr = self.force(arr_val);

                    match arr {
                        Type::ArrayRef(id) => {
                            let len = self.array_heap[id].len();
                            if idx >= len {
                                panic!("array index out of bounds: index {idx}, length {len}");
                            }

                            let elem = self.array_heap[id][idx].clone();
                            let value = self.force(elem);
                            self.stack.push(value);
                        }
                        other => {
                            panic!("type error: attempted to index non-array value {:?}", other);
                        }
                    }
                }
                Instruction::StoreIndex(name) => {
                    self.ensure_mutable_binding(&name);

                    let val = self.pop();

                    let idx_val = self.pop();
                    let idx = self.as_usize_nonneg(idx_val, "array index");

                    let target = self
                        .lookup_var(&name)
                        .cloned()
                        .unwrap_or_else(|| panic!("undefined variable: {name}"));

                    let arr = self.force(target);

                    match arr {
                        Type::ArrayRef(id) => {
                            let len = self.array_heap[id].len();
                            if idx >= len {
                                panic!("array assignment out of bounds: index {idx}, length {len}");
                            }
                            self.array_heap[id][idx] = val;
                        }
                        other => panic!("type error: StoreIndex on non-array {:?}", other),
                    }
                }
                Instruction::StoreIndexReactive(name, ast) => {
                    self.ensure_mutable_binding(&name);

                    let idx_val = self.pop();
                    let idx = self.as_usize_nonneg(idx_val, "array index");

                    let frozen = self.freeze_ast(ast);
                    let captured = self.capture_immutables_for_ast(&frozen);

                    let target = self
                        .lookup_var(&name)
                        .cloned()
                        .unwrap_or_else(|| panic!("undefined variable: {name}"));

                    let arr = self.force(target);

                    match arr {
                        Type::ArrayRef(id) => {
                            let len = self.array_heap[id].len();
                            if idx >= len {
                                panic!(
                                    "reactive array assignment out of bounds: index {idx}, length {len}"
                                );
                            }
                            self.array_heap[id][idx] = Type::LazyValue(frozen, captured);
                        }
                        other => panic!("type error: StoreIndexReactive on non-array {:?}", other),
                    }
                }

                // ----- Functions -----
                Instruction::StoreFunction(name, params, body) => {
                    self.global_env
                        .insert(name, Type::Function { params, body });
                }
                Instruction::Call(name, argc) => {
                    let args = self.pop_args(argc);

                    let f = self.global_env.get(&name).cloned().unwrap_or_else(|| {

                        panic!(
                            "call error: `{}` is not defined (attempted to call with {} argument(s))",
                            name, argc
                        )
                    });

                    match f {
                        Type::Function { .. } => {
                            let ret = self.call_function(f, args);
                            self.stack.push(ret);
                        }
                        other => panic!(
                            "call error: `{}` is not a function (found {:?})",
                            name, other
                        ),
                    }
                }

                // ----- Structs -----
                Instruction::StoreStruct(name, fields) => {
                    self.struct_defs.insert(name, fields);
                }
                Instruction::NewStruct(name) => {
                    let def = self
                        .struct_defs
                        .get(&name)
                        .cloned()
                        .unwrap_or_else(|| panic!("unknown struct type `{name}`"));
                    let inst = self.instantiate_struct(def);
                    self.stack.push(inst);
                }
                Instruction::FieldGet(field) => {
                    let obj = self.pop();
                    match self.force(obj) {
                        Type::StructRef(id) => {
                            let v = self
                                .heap
                                .get(id)
                                .unwrap_or_else(|| panic!("invalid StructRef id={id}"))
                                .fields
                                .get(&field)
                                .cloned()
                                .unwrap_or_else(|| panic!("missing struct field `{field}`"));
                            if matches!(v, Type::Uninitialized) {
                                panic!("use of uninitialized struct field `{}`", field);
                            }

                            let out = self.force_struct_field(id, v);
                            self.stack.push(out);
                        }
                        other => panic!("type error: FieldGet on non-struct {:?}", other),
                    }
                }
                Instruction::FieldSet(field) => {
                    let val = self.pop();
                    let obj = self.pop();

                    let struct_id = match self.force(obj) {
                        Type::StructRef(id) => id,
                        other => panic!("type error: FieldSet on non-struct {:?}", other),
                    };
                    {
                        let inst = &self.heap[struct_id];

                        if !inst.fields.contains_key(&field) {
                            panic!("unknown struct field `{}`", field);
                        }

                        if inst.immutables.contains(&field) {
                            panic!("cannot assign to immutable field `{}`", field);
                        }
                    }

                    let stored = self.force_to_storable(val);
                    self.heap[struct_id].fields.insert(field, stored);
                }

                Instruction::FieldSetReactive(field, ast) => {
                    let obj = self.pop();

                    match self.force(obj) {
                        Type::StructRef(id) => {
                            if self.heap[id].immutables.contains(&field) {
                                panic!("cannot reactively assign to immutable field `{}`", field);
                            }
                            let frozen = self.freeze_ast(ast);
                            let captured = self.capture_immutables_for_ast(&frozen);
                            self.heap[id]
                                .fields
                                .insert(field, Type::LazyValue(frozen, captured));
                        }
                        other => panic!("type error: FieldSetReactive on non-struct {:?}", other),
                    }
                }

                // ----- Immutable scopes -----
                Instruction::PushImmutableContext => {
                    self.immutable_stack.push(HashMap::new());
                }
                Instruction::PopImmutableContext => {
                    if self.immutable_stack.len() <= 1 {
                        panic!("internal error: cannot pop the root immutable context");
                    }
                    self.immutable_stack.pop();
                }
                Instruction::ClearImmutableContext => {
                    self.immutable_stack
                        .last_mut()
                        .expect("internal error: no immutable scope")
                        .clear();
                }
                Instruction::Label(_) => {}
                Instruction::Jump(l) => {
                    self.pointer = *self
                        .labels
                        .get(&l)
                        .unwrap_or_else(|| panic!("unknown label `{l}`"));
                    continue;
                }
                Instruction::JumpIfZero(l) => {
                    let val = self.pop();
                    let n = self.as_int(val);
                    if n == 0 {
                        self.pointer = *self
                            .labels
                            .get(&l)
                            .unwrap_or_else(|| panic!("unknown label `{l}`"));
                        continue;
                    }
                }
                Instruction::Return => {
                    return;
                }

                // ----- LValues -----
                Instruction::ArrayLValue => self.exec_array_lvalue(),
                Instruction::FieldLValue(field) => self.exec_field_lvalue(field),
                Instruction::StoreThrough => self.exec_store_through(),
                Instruction::StoreThroughReactive(ast) => self.exec_store_through_reactive(ast),
                Instruction::StoreThroughImmutable => self.store_through_immutable(),
            }

            self.pointer += 1;
        }
    }

    // =========================
    // Instruction helpers
    // =========================
    fn lookup_var(&self, name: &str) -> Option<&Type> {
        self.find_immutable(name)
            .or_else(|| self.local_env.as_ref().and_then(|e| e.get(name)))
            .or_else(|| self.global_env.get(name))
    }

    fn exec_add(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        let result = self.add_values(lhs, rhs);
        self.stack.push(result);
    }

    fn exec_sub(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        let result = self.sub_values(lhs, rhs);
        self.stack.push(result);
    }

    fn exec_mul(&mut self) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(b * a));
    }

    fn exec_div(&mut self) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(b / a));
    }

    fn exec_modulo(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        let modv = self.mod_values(lhs, rhs);
        self.stack.push(modv);
    }

    fn exec_cmp<F: FnOnce(i32, i32) -> i32>(&mut self, f: F) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(f(b, a)));
    }

    fn exec_array_lvalue(&mut self) {
        let idx_val = self.pop();
        let idx = self.as_usize_nonneg(idx_val, "array index");

        let base = self.pop();
        let base_val = self.force(base);

        match base_val {
            Type::ArrayRef(id) => {
                self.stack.push(Type::LValue(LValue::ArrayElem {
                    array_id: id,
                    index: idx,
                }));
            }

            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                let nested_val = self.array_heap[array_id][index].clone();
                let nested = self.force(nested_val);
                match nested {
                    Type::ArrayRef(nested_id) => {
                        self.stack.push(Type::LValue(LValue::ArrayElem {
                            array_id: nested_id,
                            index: idx,
                        }));
                    }
                    other => panic!("indexing non-array (found {:?})", other),
                }
            }

            Type::LValue(LValue::StructField { struct_id, field }) => {
                let field_val = self.heap[struct_id]
                    .fields
                    .get(&field)
                    .cloned()
                    .unwrap_or_else(|| panic!("missing struct field `{field}`"));

                let arr_val = self.force(field_val);
                match arr_val {
                    Type::ArrayRef(array_id) => {
                        self.stack.push(Type::LValue(LValue::ArrayElem {
                            array_id,
                            index: idx,
                        }));
                    }
                    other => panic!("indexing non-array struct field (found {:?})", other),
                }
            }

            other => panic!("invalid ArrayLValue base {:?}", other),
        }
    }

    fn exec_field_lvalue(&mut self, field: String) {
        let base = self.pop();
        match self.force(base) {
            Type::StructRef(id) => {
                self.stack.push(Type::LValue(LValue::StructField {
                    struct_id: id,
                    field,
                }));
            }

            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                let elem = self.force(self.array_heap[array_id][index].clone());
                match elem {
                    Type::StructRef(id) => {
                        self.stack.push(Type::LValue(LValue::StructField {
                            struct_id: id,
                            field,
                        }));
                    }
                    other => panic!("FieldLValue on non-struct array element {:?}", other),
                }
            }

            other => panic!("invalid FieldLValue base {:?}", other),
        }
    }

    fn exec_store_through(&mut self) {
        let value = self.pop();
        let target = self.pop();

        let stored = self.force_to_storable(value);

        match target {
            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                let len = self.array_heap[array_id].len();
                if index >= len {
                    panic!("array assignment out of bounds: index {index}, length {len}");
                }
                self.array_heap[array_id][index] = stored;
            }

            Type::LValue(LValue::StructField { struct_id, field }) => {
                let inst = &mut self.heap[struct_id];

                if !inst.fields.contains_key(&field) {
                    panic!("unknown struct field `{}`", field);
                }

                if inst.immutables.contains(&field) {
                    panic!("cannot assign to immutable field `{}`", field);
                }

                inst.fields.insert(field, stored);
            }

            other => panic!(
                "internal error: StoreThrough target is not an lvalue (got {:?})",
                other
            ),
        }
    }
    fn store_through_immutable(&mut self) {
        let value = self.pop();
        let target = self.pop();

        let stored = self.force_to_storable(value);

        match target {
            Type::LValue(LValue::StructField { struct_id, field }) => {
                let inst = &mut self.heap[struct_id];

                match inst.fields.get(&field) {
                    Some(Type::Uninitialized) => {}
                    Some(_) => panic!("cannot reassign immutable field `{}`", field),
                    None => panic!("unknown struct field `{}`", field),
                }

                inst.fields.insert(field.clone(), stored);
                inst.immutables.insert(field);
            }

            _ => panic!("immutable assignment only allowed on struct fields"),
        }
    }
    fn exec_store_through_reactive(&mut self, ast: Box<AST>) {
        let target = self.pop();

        let frozen = self.freeze_ast(ast);
        let captured = self.capture_immutables_for_ast(&frozen);

        match target {
            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                let len = self.array_heap[array_id].len();
                if index >= len {
                    panic!("reactive array assignment out of bounds: index {index}, length {len}");
                }
                self.array_heap[array_id][index] = Type::LazyValue(frozen, captured);
            }

            Type::LValue(LValue::StructField { struct_id, field }) => {
                let inst = &mut self.heap[struct_id];

                if !inst.fields.contains_key(&field) {
                    panic!("unknown struct field `{}`", field);
                }

                if inst.immutables.contains(&field) {
                    panic!("cannot reassign immutable field `{}`", field);
                }

                inst.immutables.insert(field.clone());
                inst.fields.insert(field, Type::LazyValue(frozen, captured));
            }

            other => panic!(
                "StoreThroughReactive target is not an lvalue (got {:?})",
                other
            ),
        }
    }

    // =========================
    // Runtime semantics helpers
    // =========================

    fn ensure_mutable_binding(&self, name: &str) {
        if self.immutable_exists(name) {
            panic!("cannot assign to immutable variable `{name}`");
        }
    }

    fn pop_args(&mut self, argc: usize) -> Vec<Type> {
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.pop());
        }
        args.reverse();
        args
    }

    fn print_value(&mut self, v: Type, newline: bool) {
        match self.force(v) {
            Type::Char(c) => {
                print!("{}", char::from_u32(c).unwrap());
            }
            Type::Integer(n) => {
                print!("{n}");
            }
            Type::ArrayRef(id) => {
                // Attempt to treat as string (array of chars). If not, print length.
                let elems = self.array_heap[id].clone();
                let mut all_chars = true;
                let mut chars = Vec::with_capacity(elems.len());

                for elem in elems {
                    match self.force(elem) {
                        Type::Char(c) => chars.push(c),
                        _ => {
                            all_chars = false;
                            break;
                        }
                    }
                }

                if all_chars {
                    for c in chars {
                        print!("{}", char::from_u32(c).unwrap());
                    }
                } else {
                    print!("{}", self.array_heap[id].len());
                }
            }
            other => panic!("cannot print value {:?}", other),
        }

        if newline {
            println!();
        }
    }

    fn add_values(&mut self, lhs: Type, rhs: Type) -> Type {
        match (self.force(lhs), self.force(rhs)) {
            (Type::Char(c), Type::Integer(n)) | (Type::Integer(n), Type::Char(c)) => {
                Type::Char((c as i32 + n) as u32)
            }
            (Type::Char(a), Type::Char(b)) => Type::Char((a + b) as u32),
            (a, b) => Type::Integer(self.as_int(a) + self.as_int(b)),
        }
    }

    fn sub_values(&mut self, lhs: Type, rhs: Type) -> Type {
        match (self.force(lhs), self.force(rhs)) {
            (Type::Char(c), Type::Integer(n)) | (Type::Integer(n), Type::Char(c)) => {
                Type::Char((c as i32 - n) as u32)
            }
            (Type::Char(a), Type::Char(b)) => Type::Char((a - b) as u32),
            (a, b) => Type::Integer(self.as_int(a) - self.as_int(b)),
        }
    }

    fn mod_values(&mut self, lhs: Type, rhs: Type) -> Type {
        match (self.force(lhs), self.force(rhs)) {
            (Type::Char(c), Type::Integer(n)) | (Type::Integer(n), Type::Char(c)) => {
                Type::Char((c as i32 % n) as u32)
            }
            (Type::Char(a), Type::Char(b)) => Type::Char(a % b),
            (a, b) => Type::Integer(self.as_int(a) % self.as_int(b)),
        }
    }

    fn as_usize_nonneg(&mut self, v: Type, what: &str) -> usize {
        let i = self.as_int(v);
        if i < 0 {
            panic!("{what} out of bounds: {i} is negative");
        }
        i as usize
    }

    /// Forces a value for use (pull-based reactivity):
    /// - LazyValue is evaluated
    /// - LValue is dereferenced
    /// - Everything else is returned as-is
    fn force(&mut self, v: Type) -> Type {
        match v {
            Type::LazyValue(ast, captured) => {
                self.immutable_stack.push(captured);
                let out = self.eval_value(*ast);
                self.immutable_stack.pop();
                self.force(out)
            }
            Type::LValue(lv) => {
                let val = self.read_lvalue(lv);
                self.force(val)
            }
            other => other,
        }
    }

    /// When storing into struct fields / array slots (i.e., concrete locations),
    /// we do not permit LValues to remain. LazyValue is permitted (reactive).
    fn force_to_storable(&mut self, v: Type) -> Type {
        match v {
            Type::LValue(lv) => {
                let lval = self.read_lvalue(lv);
                self.force_to_storable(lval)
            }
            Type::LazyValue(_, _) => v, // keep relationships attached to locations
            other => other,
        }
    }

    fn force_struct_field(&mut self, struct_id: usize, v: Type) -> Type {
        match v {
            Type::LazyValue(ast, captured) => {
                self.immutable_stack.push(captured);
                let out = self.eval_reactive_field_in_struct(struct_id, *ast);
                self.immutable_stack.pop();
                self.force(out)
            }
            other => self.force(other),
        }
    }

    fn read_lvalue(&mut self, lv: LValue) -> Type {
        match lv {
            LValue::ArrayElem { array_id, index } => {
                let len = self.array_heap[array_id].len();
                if index >= len {
                    panic!("array lvalue read out of bounds: index {index}, length {len}");
                }
                self.array_heap[array_id][index].clone()
            }
            LValue::StructField { struct_id, field } => self.heap[struct_id]
                .fields
                .get(&field)
                .cloned()
                .unwrap_or_else(|| panic!("missing struct field `{field}`")),
        }
    }

    fn as_int(&mut self, v: Type) -> i32 {
        match self.force(v) {
            Type::Integer(n) => n,
            Type::Char(c) => c as i32,
            Type::ArrayRef(id) => self.array_heap[id].len() as i32,
            other => panic!("type error: cannot coerce {:?} to int", other),
        }
    }

    fn pop(&mut self) -> Type {
        self.stack.pop().expect("stack underflow")
    }

    fn pop_int(&mut self) -> i32 {
        let v = self.pop();
        self.as_int(v)
    }

    // =========================
    // Struct runtime
    // =========================

    fn instantiate_struct(&mut self, fields: Vec<(String, Option<StructFieldInit>)>) -> Type {
        let mut map = HashMap::new();
        let mut imm = HashSet::new();

        // Initialize all declared fields
        // Initialize all declared fields
        for (name, init) in &fields {
            match init {
                Some(StructFieldInit::Immutable(_)) => {
                    // immutable-with-initializer: the initializer will run later, but we want the slot
                    // to exist and be considered immutable from the start.
                    imm.insert(name.clone());
                    map.insert(name.clone(), Type::Uninitialized);
                }
                Some(StructFieldInit::Reactive(_)) => {
                    // reactive initializer stored later; slot exists now
                    map.insert(
                        name.clone(),
                        Type::LazyValue(Box::new(AST::Number(0)), HashMap::new()),
                    );
                }
                Some(StructFieldInit::Mutable(_)) => {
                    // will be initialized later
                    map.insert(name.clone(), Type::Uninitialized);
                }
                None => {
                    // key fix: bare `x;` starts uninitialized, so `x := ...` can be a one-time init
                    map.insert(name.clone(), Type::Uninitialized);
                }
            }
        }

        let id = self.heap.len();
        self.heap.push(StructInstance {
            fields: map,
            immutables: imm.clone(),
        });

        // Apply initializers (mutable/immutable are eager; reactive stores relationship)
        for (name, init) in fields {
            if let Some(init) = init {
                let value = match init {
                    StructFieldInit::Mutable(ast) | StructFieldInit::Immutable(ast) => {
                        self.eval_reactive_field_in_struct(id, ast)
                    }
                    StructFieldInit::Reactive(ast) => {
                        let frozen = self.freeze_ast(Box::new(ast));
                        let captured = self.capture_immutables_for_ast(&frozen);
                        Type::LazyValue(frozen, captured)
                    }
                };

                let stored = self.force_to_storable(value);
                let cloned = self.clone_value(stored);
                self.heap[id].fields.insert(name, cloned);
            }
        }

        Type::StructRef(id)
    }

    fn eval_reactive_field_in_struct(&mut self, struct_id: usize, ast: AST) -> Type {
        // Each evaluation creates a fresh immutable frame and binds all fields as LValues.
        self.immutable_stack.push(HashMap::new());

        {
            let scope = self
                .immutable_stack
                .last_mut()
                .expect("internal error: no immutable scope for struct eval");
            let keys: Vec<String> = self.heap[struct_id].fields.keys().cloned().collect();
            for key in keys {
                scope.insert(
                    key.clone(),
                    Type::LValue(LValue::StructField {
                        struct_id,
                        field: key,
                    }),
                );
            }
        }

        let result = self.eval_value(ast);
        self.immutable_stack.pop();
        result
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
            Type::LazyValue(ast, captured) => Type::LazyValue(ast, captured),
            Type::Integer(n) => Type::Integer(n),
            Type::Function { params, body } => Type::Function { params, body },
            Type::LValue(_) => panic!("cannot clone lvalue"),
            Type::Char(c) => Type::Char(c),
            Type::Uninitialized => Type::Uninitialized,
        }
    }

    // =========================
    // Function calls / modules
    // =========================

    fn call_function(&mut self, f: Type, args: Vec<Type>) -> Type {
        match f {
            Type::Function { params, body } => {
                let saved_local = self.local_env.take();
                let saved_immutables = self.immutable_stack.clone();
                self.immutable_stack.push(HashMap::new());

                self.local_env = Some(HashMap::new());

                {
                    let scope = self.immutable_stack.last_mut().unwrap();
                    for (p, v) in params.into_iter().zip(args) {
                        scope.insert(p, v);
                    }
                }

                let mut code = Vec::new();
                let mut lg = crate::compiler::LabelGenerator::new();
                let mut bs = Vec::new();
                for stmt in body {
                    crate::compiler::compile(stmt, &mut code, &mut lg, &mut bs);
                }
                code.push(Instruction::Return);

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
                self.immutable_stack = saved_immutables;
                self.local_env = saved_local;

                ret
            }
            _ => panic!("attempted to call non-function"),
        }
    }

    fn import_module(&mut self, path: Vec<String>) {
        let file_path = format!("project/{}.rx", path.join("/"));

        let source = std::fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("could not import module `{}`", file_path));

        let tokens = crate::tokenizer::tokenize(&source);
        let ast = crate::parser::parse(tokens);

        let mut code = Vec::new();
        let mut lg = crate::compiler::LabelGenerator::new();
        let mut bs = Vec::new();
        crate::compiler::compile_module(ast, &mut code, &mut lg, &mut bs);

        let saved_code = std::mem::replace(&mut self.code, code);
        let saved_labels = std::mem::replace(&mut self.labels, Self::build_labels(&self.code));
        let saved_ptr = self.pointer;

        self.pointer = 0;
        self.run();

        self.code = saved_code;
        self.labels = saved_labels;
        self.pointer = saved_ptr;
    }

    // =========================
    // Lazy/reactive evaluation (AST interpreter)
    // =========================

    fn eval_value(&mut self, ast: AST) -> Type {
        match ast {
            AST::Number(n) => Type::Integer(n),
            AST::Char(c) => Type::Char(c),

            AST::Var(name) => {
                if let Some(v) = self.find_immutable(&name).cloned() {
                    return v;
                }
                if let Some(v) = self.lookup_var(&name).cloned() {
                    return v;
                }

                self.dbg_dump_state(&format!(
                    "UNBOUND VAR lookup failed for `{}` while eval_value(Var)",
                    name
                ));
                panic!("undefined variable: {name}");
            }

            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let value = self.eval_value(*cond);
                let c = self.as_int(value);
                if c != 0 {
                    self.eval_value(*then_expr)
                } else {
                    self.eval_value(*else_expr)
                }
            }

            AST::ArrayNew(size_ast) => {
                let value = self.eval_value(*size_ast);
                let n = self.as_usize_nonneg(value, "array size");
                let id = self.array_heap.len();
                self.array_heap.push(vec![Type::Integer(0); n]);
                Type::ArrayRef(id)
            }

            AST::StringLiteral(s) => {
                let id = self.array_heap.len();
                let mut arr = Vec::with_capacity(s.chars().count());
                for c in s.chars() {
                    arr.push(Type::Char(c as u32));
                }
                self.array_heap.push(arr);
                Type::ArrayRef(id)
            }

            AST::FieldAccess(base, field) => {
                let value = self.eval_value(*base);
                let obj = self.force(value);
                match obj {
                    Type::StructRef(id) => {
                        let v = self
                            .heap
                            .get(id)
                            .and_then(|s| s.fields.get(&field))
                            .cloned()
                            .unwrap_or_else(|| panic!("missing struct field `{field}`"));
                        self.force_struct_field(id, v)
                    }
                    other => panic!("type error: field access on non-struct {:?}", other),
                }
            }

            AST::Index(base, index) => {
                let idx_val = self.eval_value(*index);
                let idx = self.as_usize_nonneg(idx_val, "array index");

                let base_val = self.eval_value(*base);
                let arr = self.force(base_val);

                match arr {
                    Type::ArrayRef(id) => {
                        let len = self.array_heap[id].len();
                        if idx >= len {
                            panic!("array index out of bounds: index {idx}, length {len}");
                        }
                        let elem = self.array_heap[id][idx].clone();
                        self.force(elem)
                    }
                    other => panic!("type error: indexing non-array {:?}", other),
                }
            }

            AST::Call { name, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_value(a));
                }
                let f = self
                    .global_env
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| panic!("call error: `{name}` is not defined"));

                self.call_function(f, vals)
            }

            AST::Operation(l, op, r) => {
                let lv = self.eval_value(*l);
                let rv = self.eval_value(*r);

                // Preserve your char+int char semantics.
                match (&op, self.force(lv), self.force(rv)) {
                    (Operator::Addition, Type::Char(c), Type::Integer(n))
                    | (Operator::Addition, Type::Integer(n), Type::Char(c)) => {
                        Type::Char((c as i32 + n) as u32)
                    }
                    (Operator::Subtraction, Type::Char(c), Type::Integer(n))
                    | (Operator::Subtraction, Type::Integer(n), Type::Char(c)) => {
                        Type::Char((c as i32 - n) as u32)
                    }
                    (Operator::Modulo, Type::Char(c), Type::Integer(n))
                    | (Operator::Modulo, Type::Integer(n), Type::Char(c)) => {
                        Type::Char((c as i32 % n) as u32)
                    }
                    (op, a, b) => {
                        let ai = self.as_int(a);
                        let bi = self.as_int(b);
                        Type::Integer(match op {
                            Operator::Addition => ai + bi,
                            Operator::Subtraction => ai - bi,
                            Operator::Multiplication => ai * bi,
                            Operator::Division => ai / bi,
                            Operator::Greater => (ai > bi) as i32,
                            Operator::Less => (ai < bi) as i32,
                            Operator::Equal => (ai == bi) as i32,
                            Operator::NotEqual => (ai != bi) as i32,
                            Operator::GreaterEqual => (ai >= bi) as i32,
                            Operator::LessEqual => (ai <= bi) as i32,
                            Operator::And => ((ai > 0) && (bi > 0)) as i32,
                            Operator::Or => ((ai > 0) || (bi > 0)) as i32,
                            Operator::Modulo => ai % bi,
                        })
                    }
                }
            }

            other => panic!("eval_value(): unsupported AST variant: {:?}", other),
        }
    }

    // =========================
    // Reactive capture utilities
    // =========================

    fn ast_free_vars(&self, ast: &AST, out: &mut HashSet<String>) {
        match ast {
            AST::Var(n) => {
                out.insert(n.clone());
            }
            AST::Operation(l, _, r) => {
                self.ast_free_vars(l, out);
                self.ast_free_vars(r, out);
            }
            AST::Index(b, i) => {
                self.ast_free_vars(b, out);
                self.ast_free_vars(i, out);
            }
            AST::FieldAccess(b, _) => {
                self.ast_free_vars(b, out);
            }
            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.ast_free_vars(cond, out);
                self.ast_free_vars(then_expr, out);
                self.ast_free_vars(else_expr, out);
            }
            AST::Call { args, .. } => {
                for a in args {
                    self.ast_free_vars(a, out);
                }
            }
            AST::Number(_) | AST::Char(_) | AST::StringLiteral(_) => {}
            _ => {}
        }
    }

    fn capture_immutables_for_ast(&self, ast: &AST) -> HashMap<String, Type> {
        let mut names = HashSet::new();
        self.ast_free_vars(ast, &mut names);

        let mut cap = HashMap::new();
        for n in names {
            if let Some(v) = self.find_immutable(&n).cloned() {
                cap.insert(n, v);
            }
        }
        cap
    }

    // Freeze immutables that are integers.
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

    // =========================
    // Debugging
    // =========================

    fn dbg_short_type(&self, v: &Type) -> String {
        match v {
            Type::Integer(n) => format!("Int({})", n),
            Type::Char(c) => format!("Char({})", c),
            Type::ArrayRef(id) => format!("ArrayRef({})", id),
            Type::StructRef(id) => format!("StructRef({})", id),
            Type::Function { params, .. } => format!("Function(params={:?})", params),
            Type::LValue(lv) => format!("LValue({:?})", lv),
            Type::LazyValue(ast, captured) => format!("Lazy({:?}, cap={:?})", ast, captured.keys()),
            Type::Uninitialized => "Uninitialized".to_string(),
        }
    }

    fn dump_env_keys(&self) -> Vec<String> {
        let mut keys: Vec<_> = self.global_env.keys().cloned().collect();

        keys.sort();
        keys
    }

    fn dump_stack(&self) -> Vec<String> {
        self.stack.iter().map(|v| self.dbg_short_type(v)).collect()
    }

    fn dbg_dump_state(&self, headline: &str) {
        if !self.debug {
            return;
        }

        eprintln!("\n================ VM DEBUG ================");
        eprintln!("{}", headline);
        eprintln!(
            "ip={} instr={:?}",
            self.pointer,
            self.code.get(self.pointer)
        );
        eprintln!("reactive_ctx={:?}", self.debug_reactive_ctx);
        eprintln!("stack(len={}): {:?}", self.stack.len(), self.dump_stack());
        eprintln!("env keys: {:?}", self.dump_env_keys());
        eprintln!("immutable frames: {}", self.immutable_stack.len());
        for (frame_i, scope) in self.immutable_stack.iter().enumerate() {
            let mut keys: Vec<_> = scope.keys().cloned().collect();
            keys.sort();
            eprintln!("  frame[{frame_i}] keys={keys:?}");
        }
        eprintln!("heap structs: {}", self.heap.len());
        eprintln!("array heap: {}", self.array_heap.len());
        eprintln!("==========================================\n");
    }
}
