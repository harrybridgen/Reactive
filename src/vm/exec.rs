use std::collections::HashSet;

use super::VM;
use crate::grammar::{CastType, Instruction, LValue, ReactiveExpr, Type};

impl VM {
    pub fn run(&mut self) {
        while self.pointer < self.code.len() {
            let instr = self.code[self.pointer].clone();

            match instr {
                Instruction::Push(n) => self.stack.push(Type::Integer(n)),
                Instruction::PushChar(c) => self.stack.push(Type::Char(c)),
                Instruction::Load(name) => self.exec_load(name),
                Instruction::Store(name) => self.exec_store(name),
                Instruction::StoreImmutable(name) => self.exec_store_immutable(name),
                Instruction::StoreReactive(name, expr) => self.exec_store_reactive(name, expr),
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
                Instruction::Print => self.exec_print(),
                Instruction::Println => self.exec_print(),
                Instruction::Assert => self.exec_assert(),
                Instruction::Error(message) => self.exec_error(message),
                Instruction::ArrayNew => self.exec_array_new(),
                Instruction::ArrayGet => self.exec_array_get(),
                Instruction::StoreIndex(name) => self.exec_store_index(name),
                Instruction::StoreIndexReactive(name, expr) => {
                    self.exec_store_index_reactive(name, expr)
                }
                Instruction::StoreFunction(name, params, body) => {
                    self.global_env
                        .insert(name, Type::Function { params, code: body });
                }
                Instruction::Call(name, argc) => self.exec_call(name, argc),
                Instruction::StoreStruct(name, fields) => {
                    self.struct_defs.insert(name, fields);
                }
                Instruction::NewStruct(name) => self.exec_new_struct(name),
                Instruction::FieldGet(field) => self.exec_field_get(field),
                Instruction::FieldSet(field) => self.exec_field_set(field),
                Instruction::FieldSetReactive(field, expr) => {
                    self.exec_field_set_reactive(field, expr)
                }
                Instruction::PushImmutableContext => {
                    self.immutable_stack.push(std::collections::HashMap::new());
                }
                Instruction::PopImmutableContext => {
                    if self.immutable_stack.len() <= 1 {
                        self.runtime_error("internal error: cannot pop root immutable context");
                    }
                    self.immutable_stack.pop();
                }
                Instruction::ClearImmutableContext => {
                    if let Some(scope) = self.immutable_stack.last_mut() {
                        scope.clear();
                    } else {
                        self.runtime_error("internal error: no immutable scope");
                    }
                }
                Instruction::Label(_) => {}
                Instruction::Jump(label) => {
                    self.pointer = *self
                        .labels
                        .get(&label)
                        .unwrap_or_else(|| self.runtime_error(&format!("unknown label `{label}`")));
                    continue;
                }
                Instruction::JumpIfZero(label) => {
                    let n = self.pop_int();
                    if n == 0 {
                        self.pointer = *self.labels.get(&label).unwrap_or_else(|| {
                            self.runtime_error(&format!("unknown label `{label}`"))
                        });
                        continue;
                    }
                }
                Instruction::Return => return,
                Instruction::ArrayLValue => self.exec_array_lvalue(),
                Instruction::FieldLValue(field) => self.exec_field_lvalue(field),
                Instruction::StoreThrough => self.exec_store_through(),
                Instruction::StoreThroughReactive(expr) => self.exec_store_through_reactive(expr),
                Instruction::StoreThroughImmutable => self.store_through_immutable(),
                Instruction::Import(path) => self.exec_import(path),
                Instruction::Cast(target) => self.exec_cast(target),
            }

            self.pointer += 1;
        }
    }
    pub(crate) fn exec_call(&mut self, name: String, argc: usize) {
        let args = self.pop_args(argc);

        let f = self.global_env.get(&name).cloned().unwrap_or_else(|| {
            self.runtime_error(&format!(
                "call error: `{}` is not defined (attempted to call with {} argument(s))",
                name, argc
            ))
        });

        let ret = match f {
            Type::Function { .. } => self.call_function(name, f, args),
            Type::NativeFunction(native_name) => self.call_native(native_name, args),
            other => self.runtime_error(&format!(
                "call error: `{}` is not a function (found {:?})",
                name, other
            )),
        };

        self.stack.push(ret);
    }
    pub(crate) fn exec_field_get(&mut self, field: String) {
        let obj = self.pop();
        match self.force(obj) {
            Type::StructRef(id) => {
                let v = self
                    .heap
                    .get(id)
                    .unwrap_or_else(|| self.runtime_error(&format!("invalid StructRef id={id}")))
                    .fields
                    .get(&field)
                    .cloned()
                    .unwrap_or_else(|| {
                        self.runtime_error(&format!("missing struct field `{field}`"))
                    });

                if matches!(v, Type::Uninitialized) {
                    self.runtime_error(&format!("use of uninitialized struct field `{}`", field));
                }

                let out = self.force_struct_field(id, v);
                self.stack.push(out);
            }
            other => self.runtime_error(&format!("type error: FieldGet on non-struct {:?}", other)),
        }
    }

    pub(crate) fn exec_field_set(&mut self, field: String) {
        let val = self.pop();
        let obj = self.pop();

        let struct_id = match self.force(obj) {
            Type::StructRef(id) => id,
            other => self.runtime_error(&format!("type error: FieldSet on non-struct {:?}", other)),
        };

        {
            let inst = &self.heap[struct_id];

            if !inst.fields.contains_key(&field) {
                self.runtime_error(&format!("unknown struct field `{}`", field));
            }

            if inst.immutables.contains(&field) {
                self.runtime_error(&format!("cannot assign to immutable field `{}`", field));
            }
        }

        let stored = self.force_to_storable(val);
        self.heap[struct_id].fields.insert(field, stored);
    }

    pub(crate) fn exec_field_set_reactive(&mut self, field: String, expr: ReactiveExpr) {
        let obj = self.pop();

        match self.force(obj) {
            Type::StructRef(id) => {
                if self.heap[id].immutables.contains(&field) {
                    self.runtime_error(&format!(
                        "cannot reactively assign to immutable field `{}`",
                        field
                    ));
                }
                let captured = self.capture_immutables(&expr.captures);
                self.heap[id]
                    .fields
                    .insert(field, Type::LazyValue(expr, captured));
            }
            other => self.runtime_error(&format!(
                "type error: FieldSetReactive on non-struct {:?}",
                other
            )),
        }
    }
    pub(crate) fn exec_array_lvalue(&mut self) {
        let idx_val = self.pop();
        let idx = self.as_usize(idx_val, "array index");

        let base = self.pop();
        let base_val = self.force(base);

        match base_val {
            Type::ArrayRef(id) => {
                self.stack.push(Type::LValue(LValue::ArrayElem {
                    array_id: id,
                    index: idx,
                }));
            }
            Type::VecRef(id) => {
                self.stack.push(Type::LValue(LValue::VecElem {
                    vec_id: id,
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
                    Type::VecRef(nested_id) => {
                        self.stack.push(Type::LValue(LValue::VecElem {
                            vec_id: nested_id,
                            index: idx,
                        }));
                    }
                    other => self.runtime_error(&format!("indexing non-array (found {:?})", other)),
                }
            }
            Type::LValue(LValue::VecElem { vec_id, index }) => {
                let nested_val = self.vec_heap[vec_id][index].clone();
                let nested = self.force(nested_val);
                match nested {
                    Type::ArrayRef(array_id) => {
                        self.stack.push(Type::LValue(LValue::ArrayElem {
                            array_id,
                            index: idx,
                        }));
                    }
                    Type::VecRef(nested_id) => {
                        self.stack.push(Type::LValue(LValue::VecElem {
                            vec_id: nested_id,
                            index: idx,
                        }));
                    }
                    other => self.runtime_error(&format!("indexing non-array (found {:?})", other)),
                }
            }

            Type::LValue(LValue::StructField { struct_id, field }) => {
                let field_val = self.heap[struct_id]
                    .fields
                    .get(&field)
                    .cloned()
                    .unwrap_or_else(|| {
                        self.runtime_error(&format!("missing struct field `{field}`"))
                    });

                let arr_val = self.force(field_val);
                match arr_val {
                    Type::ArrayRef(array_id) => {
                        self.stack.push(Type::LValue(LValue::ArrayElem {
                            array_id,
                            index: idx,
                        }));
                    }
                    Type::VecRef(vec_id) => {
                        self.stack
                            .push(Type::LValue(LValue::VecElem { vec_id, index: idx }));
                    }
                    other => self.runtime_error(&format!(
                        "indexing non-array struct field (found {:?})",
                        other
                    )),
                }
            }

            other => self.runtime_error(&format!("invalid ArrayLValue base {:?}", other)),
        }
    }

    pub(crate) fn exec_field_lvalue(&mut self, field: String) {
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
                    other => self.runtime_error(&format!(
                        "FieldLValue on non-struct array element {:?}",
                        other
                    )),
                }
            }
            Type::LValue(LValue::VecElem { vec_id, index }) => {
                let elem = self.force(self.vec_heap[vec_id][index].clone());
                match elem {
                    Type::StructRef(id) => {
                        self.stack.push(Type::LValue(LValue::StructField {
                            struct_id: id,
                            field,
                        }));
                    }
                    other => self.runtime_error(&format!(
                        "FieldLValue on non-struct vec element {:?}",
                        other
                    )),
                }
            }

            other => self.runtime_error(&format!("invalid FieldLValue base {:?}", other)),
        }
    }

    pub(crate) fn exec_store_through(&mut self) {
        let value = self.pop();
        let target = self.pop();

        let stored = self.force_to_storable(value);

        match target {
            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                if self.array_immutables[array_id].contains(&index) {
                    self.runtime_error("cannot reassign immutable array element");
                }

                let len = self.array_heap[array_id].len();
                if index >= len {
                    self.runtime_error("array assignment out of bounds");
                }

                self.array_heap[array_id][index] = stored;
            }
            Type::LValue(LValue::VecElem { vec_id, index }) => {
                if self.vec_immutables[vec_id].contains(&index) {
                    self.runtime_error("cannot reassign immutable vec element");
                }

                let len = self.vec_heap[vec_id].len();
                if index >= len {
                    self.runtime_error("vec assignment out of bounds");
                }

                self.vec_heap[vec_id][index] = stored;
            }

            Type::LValue(LValue::StructField { struct_id, field }) => {
                let inst = &mut self.heap[struct_id];

                if !inst.fields.contains_key(&field) {
                    self.runtime_error(&format!("unknown struct field `{}`", field));
                }

                if inst.immutables.contains(&field) {
                    self.runtime_error(&format!("cannot assign to immutable field `{}`", field));
                }

                inst.fields.insert(field, stored);
            }

            other => self.runtime_error(&format!(
                "internal error: StoreThrough target is not an lvalue (got {:?})",
                other
            )),
        }
    }

    pub(crate) fn exec_store_through_reactive(&mut self, expr: ReactiveExpr) {
        let target = self.pop();

        let captured = self.capture_immutables(&expr.captures);
        let value = Type::LazyValue(expr, captured);

        match target {
            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                if self.array_immutables[array_id].contains(&index) {
                    self.runtime_error("cannot reassign immutable array element");
                }

                let len = self.array_heap[array_id].len();
                if index >= len {
                    self.runtime_error("reactive array assignment out of bounds");
                }

                self.array_heap[array_id][index] = value;
            }
            Type::LValue(LValue::VecElem { vec_id, index }) => {
                if self.vec_immutables[vec_id].contains(&index) {
                    self.runtime_error("cannot reassign immutable vec element");
                }

                let len = self.vec_heap[vec_id].len();
                if index >= len {
                    self.runtime_error("reactive vec assignment out of bounds");
                }

                self.vec_heap[vec_id][index] = value;
            }

            Type::LValue(LValue::StructField { struct_id, field }) => {
                let inst = &mut self.heap[struct_id];

                if !inst.fields.contains_key(&field) {
                    self.runtime_error(&format!("unknown struct field `{}`", field));
                }

                if inst.immutables.contains(&field) {
                    self.runtime_error(&format!("cannot reassign immutable field `{}`", field));
                }

                inst.immutables.insert(field.clone());
                inst.fields.insert(field, value);
            }

            other => self.runtime_error(&format!(
                "StoreThroughReactive target is not an lvalue (got {:?})",
                other
            )),
        }
    }
    pub(crate) fn exec_array_new(&mut self) {
        let size_val = self.pop();
        let n = self.as_usize(size_val, "array size");

        let id = self.array_heap.len();
        self.array_heap.push(vec![Type::Integer(0); n]);
        self.array_immutables.push(HashSet::new());
        self.stack.push(Type::ArrayRef(id));
    }

    pub(crate) fn exec_array_get(&mut self) {
        let idx_val = self.pop();
        let idx = self.as_usize(idx_val, "array index");

        let arr_val = self.pop();
        let arr = self.force(arr_val);

        match arr {
            Type::ArrayRef(id) => {
                let len = self.array_heap[id].len();
                if idx >= len {
                    self.runtime_error(&format!(
                        "array index out of bounds: index {idx}, length {len}"
                    ));
                }
                let elem = self.array_heap[id][idx].clone();
                let f = self.force(elem);
                self.stack.push(f);
            }
            Type::VecRef(id) => {
                let len = self.vec_heap[id].len();
                if idx >= len {
                    self.runtime_error(&format!(
                        "vec index out of bounds: index {idx}, length {len}"
                    ));
                }
                let elem = self.vec_heap[id][idx].clone();
                let f = self.force(elem);
                self.stack.push(f);
            }
            other => self.runtime_error(&format!(
                "type error: attempted to index non-array value {:?}",
                other
            )),
        }
    }

    pub(crate) fn exec_store_index(&mut self, name: String) {
        self.ensure_mutable_binding(&name);

        let val = self.pop();

        let idx_val = self.pop();
        let idx = self.as_usize(idx_val, "array index");

        let target = self
            .lookup_var(&name)
            .cloned()
            .unwrap_or_else(|| self.runtime_error(&format!("undefined variable: {name}")));

        let arr = self.force(target);

        match arr {
            Type::ArrayRef(id) => {
                let len = self.array_heap[id].len();
                if idx >= len {
                    self.runtime_error(&format!(
                        "array assignment out of bounds: index {idx}, length {len}"
                    ));
                }
                self.array_heap[id][idx] = val;
            }
            Type::VecRef(id) => {
                let len = self.vec_heap[id].len();
                if idx >= len {
                    self.runtime_error(&format!(
                        "vec assignment out of bounds: index {idx}, length {len}"
                    ));
                }
                self.vec_heap[id][idx] = val;
            }
            other => {
                self.runtime_error(&format!("type error: StoreIndex on non-array {:?}", other))
            }
        }
    }

    pub(crate) fn exec_store_index_reactive(&mut self, name: String, expr: ReactiveExpr) {
        self.ensure_mutable_binding(&name);

        let idx_val = self.pop();
        let idx = self.as_usize(idx_val, "array index");

        let captured = self.capture_immutables(&expr.captures);
        let value = Type::LazyValue(expr, captured);

        let target = self
            .lookup_var(&name)
            .cloned()
            .unwrap_or_else(|| self.runtime_error(&format!("undefined variable: {name}")));

        let arr = self.force(target);

        match arr {
            Type::ArrayRef(id) => {
                let len = self.array_heap[id].len();
                if idx >= len {
                    self.runtime_error(&format!(
                        "reactive array assignment out of bounds: index {idx}, length {len}"
                    ));
                }
                self.array_heap[id][idx] = value;
            }
            Type::VecRef(id) => {
                let len = self.vec_heap[id].len();
                if idx >= len {
                    self.runtime_error(&format!(
                        "reactive vec assignment out of bounds: index {idx}, length {len}"
                    ));
                }
                self.vec_heap[id][idx] = value;
            }
            other => self.runtime_error(&format!(
                "type error: StoreIndexReactive on non-array {:?}",
                other
            )),
        }
    }

    fn exec_new_struct(&mut self, name: String) {
        let def = self
            .struct_defs
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.runtime_error(&format!("unknown struct type `{name}`")));
        let inst = self.instantiate_struct(def);
        self.stack.push(inst);
    }

    fn exec_import(&mut self, path: Vec<String>) {
        let module_name = path.join(".");
        if !self.imported_modules.contains(&module_name) {
            self.imported_modules.insert(module_name.clone());
            self.import_module(path);
        }
    }

    fn exec_cast(&mut self, target: CastType) {
        let v = self.pop();
        match target {
            CastType::Int => {
                let n = self.as_int(v);
                self.stack.push(Type::Integer(n));
            }
            CastType::Char => {
                let n = self.as_int(v);
                if n < 0 || n > 0x10FFFF {
                    self.runtime_error(&format!("invalid char code {}", n));
                }
                self.stack.push(Type::Char(n as u32));
            }
        }
    }

    fn exec_error(&mut self, message: String) {
        self.runtime_error(&message);
    }

    fn exec_assert(&mut self) {
        let v = self.pop_int();
        if v == 0 {
            self.runtime_error("assertion failed");
        }
    }

    fn exec_print(&mut self) {
        let v = self.pop();
        self.print_value(v, false);
    }

    fn exec_load(&mut self, name: String) {
        let v = self
            .lookup_var(&name)
            .cloned()
            .unwrap_or_else(|| self.runtime_error(&format!("undefined variable: {name}")));
        let value = self.force(v);
        self.stack.push(value);
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
        let scope = match self.immutable_stack.last_mut() {
            Some(scope) => scope,
            None => self.runtime_error("internal error: no immutable scope"),
        };
        if scope.contains_key(&name) {
            self.runtime_error(&format!("cannot reassign immutable variable `{name}`"));
        }
        scope.insert(name, v);
    }

    fn exec_store_reactive(&mut self, name: String, expr: ReactiveExpr) {
        self.ensure_mutable_binding(&name);
        let captured = self.capture_immutables(&expr.captures);
        let value = Type::LazyValue(expr, captured);

        match &mut self.local_env {
            Some(env) => {
                env.insert(name, value);
            }
            None => {
                self.global_env.insert(name, value);
            }
        }
    }

    // =========================================================
    // Arithmetic / comparisons
    // =========================================================

    fn exec_add(&mut self) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(b + a));
    }

    fn exec_sub(&mut self) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(b - a));
    }

    fn exec_modulo(&mut self) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(b % a));
    }
    fn exec_mul(&mut self) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(b * a));
    }

    fn exec_div(&mut self) {
        let a = self.pop_int();
        if a == 0 {
            self.runtime_error("division by zero");
        }
        let b = self.pop_int();
        self.stack.push(Type::Integer(b / a));
    }

    fn exec_cmp<F: FnOnce(i32, i32) -> i32>(&mut self, f: F) {
        let a = self.pop_int();
        let b = self.pop_int();
        self.stack.push(Type::Integer(f(b, a)));
    }
}
