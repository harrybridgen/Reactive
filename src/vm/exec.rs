use super::VM;
use crate::grammar::{AST, Instruction, Type};

impl VM {
    pub fn run(&mut self) {
        while self.pointer < self.code.len() {
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
                Instruction::Store(name) => self.exec_store(name),
                Instruction::StoreImmutable(name) => self.exec_store_immutable(name),
                Instruction::StoreReactive(name, ast) => self.exec_store_reactive(name, ast),
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
                Instruction::Print => {
                    let v = self.pop();
                    self.print_value(v, false);
                }
                Instruction::Println => {
                    let v = self.pop();
                    self.print_value(v, true);
                }
                Instruction::ArrayNew => self.exec_array_new(),
                Instruction::ArrayGet => self.exec_array_get(),
                Instruction::StoreIndex(name) => self.exec_store_index(name),
                Instruction::StoreIndexReactive(name, ast) => {
                    self.exec_store_index_reactive(name, ast)
                }
                Instruction::StoreFunction(name, params, body) => {
                    self.global_env
                        .insert(name, Type::Function { params, body });
                }
                Instruction::Call(name, argc) => self.exec_call(name, argc),
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
                Instruction::FieldGet(field) => self.exec_field_get(field),
                Instruction::FieldSet(field) => self.exec_field_set(field),
                Instruction::FieldSetReactive(field, ast) => {
                    self.exec_field_set_reactive(field, ast)
                }
                Instruction::PushImmutableContext => {
                    self.immutable_stack.push(std::collections::HashMap::new());
                }
                Instruction::PopImmutableContext => {
                    if self.immutable_stack.len() <= 1 {
                        panic!("internal error: cannot pop root immutable context");
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
                Instruction::Jump(label) => {
                    self.pointer = *self
                        .labels
                        .get(&label)
                        .unwrap_or_else(|| panic!("unknown label `{label}`"));
                    continue;
                }
                Instruction::JumpIfZero(label) => {
                    let n = self.pop_int();
                    if n == 0 {
                        self.pointer = *self
                            .labels
                            .get(&label)
                            .unwrap_or_else(|| panic!("unknown label `{label}`"));
                        continue;
                    }
                }
                Instruction::Return => return,
                Instruction::ArrayLValue => self.exec_array_lvalue(),
                Instruction::FieldLValue(field) => self.exec_field_lvalue(field),
                Instruction::StoreThrough => self.exec_store_through(),
                Instruction::StoreThroughReactive(ast) => self.exec_store_through_reactive(ast),
                Instruction::StoreThroughImmutable => self.store_through_immutable(),
                Instruction::Import(path) => {
                    let module_name = path.join(".");
                    if !self.imported_modules.contains(&module_name) {
                        self.imported_modules.insert(module_name.clone());
                        self.import_module(path);
                    }
                }
            }

            self.pointer += 1;
        }
    }

    // =========================================================
    // Store handlers
    // =========================================================
    fn exec_store(&mut self, name: String) {
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

    fn exec_store_immutable(&mut self, name: String) {
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

    fn exec_store_reactive(&mut self, name: String, ast: Box<AST>) {
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

    // =========================================================
    // Arithmetic / comparisons
    // =========================================================

    fn exec_add(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        let value = self.add_values(lhs, rhs);
        self.stack.push(value);
    }

    fn exec_sub(&mut self) {
        let rhs = self.pop();
        let lhs = self.pop();
        let value = self.sub_values(lhs, rhs);
        self.stack.push(value);
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
        let modu = self.mod_values(lhs, rhs);
        self.stack.push(modu);
    }

    fn exec_cmp<F: FnOnce(i32, i32) -> i32>(&mut self, f: F) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(f(b, a)));
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
}
