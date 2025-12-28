use super::VM;
use crate::grammar::{
    CompiledStructFieldInit, Instruction, LValue, ReactiveExpr, StructInstance, Type,
};
use std::collections::{HashMap, HashSet};

impl VM {
    // =========================================================
    // Stack helpers
    // =========================================================

    pub(crate) fn pop(&mut self) -> Type {
        match self.stack.pop() {
            Some(value) => value,
            None => self.runtime_error("stack underflow"),
        }
    }

    pub(crate) fn pop_int(&mut self) -> i32 {
        let v = self.pop();
        self.as_int(v)
    }

    pub(crate) fn pop_args(&mut self, argc: usize) -> Vec<Type> {
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.pop());
        }
        args.reverse();
        args
    }

    // =========================================================
    // Coercions / bounds
    // =========================================================

    pub(crate) fn as_int(&mut self, v: Type) -> i32 {
        match self.force(v) {
            Type::Integer(n) => n,
            Type::Char(c) => c as i32,
            Type::ArrayRef(id) => self.array_heap[id].len() as i32,
            other => self.runtime_error(&format!("type error: cannot coerce {:?} to int", other)),
        }
    }

    pub(crate) fn as_usize_nonneg(&mut self, v: Type, what: &str) -> usize {
        let i = self.as_int(v);
        if i < 0 {
            self.runtime_error(&format!("{what} out of bounds: {i} is negative"));
        }
        i as usize
    }

    // =========================================================
    // Printing
    // =========================================================

    pub(crate) fn print_value(&mut self, v: Type, newline: bool) {
        match self.force(v) {
            Type::Char(c) => {
                print!("{}", char::from_u32(c).unwrap());
            }
            Type::Integer(n) => {
                print!("{n}");
            }
            Type::ArrayRef(id) => {
                // Attempt to treat as string (array of chars). If not, print length
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
            other => self.runtime_error(&format!("cannot print value {:?}", other)),
        }

        if newline {
            println!();
        }
    }

    // =========================================================
    // Arrays
    // =========================================================

    pub(crate) fn exec_array_new(&mut self) {
        let size_val = self.pop();
        let n = self.as_usize_nonneg(size_val, "array size");

        let id = self.array_heap.len();
        self.array_heap.push(vec![Type::Integer(0); n]);
        self.array_immutables.push(HashSet::new());
        self.stack.push(Type::ArrayRef(id));
    }

    pub(crate) fn exec_array_get(&mut self) {
        let idx_val = self.pop();
        let idx = self.as_usize_nonneg(idx_val, "array index");

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
        let idx = self.as_usize_nonneg(idx_val, "array index");

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
            other => self.runtime_error(&format!("type error: StoreIndex on non-array {:?}", other)),
        }
    }

    pub(crate) fn exec_store_index_reactive(&mut self, name: String, expr: ReactiveExpr) {
        self.ensure_mutable_binding(&name);

        let idx_val = self.pop();
        let idx = self.as_usize_nonneg(idx_val, "array index");

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
            other => self.runtime_error(&format!(
                "type error: StoreIndexReactive on non-array {:?}",
                other
            )),
        }
    }

    // =========================================================
    // LValues
    // =========================================================

    pub(crate) fn read_lvalue(&mut self, lv: LValue) -> Type {
        match lv {
            LValue::ArrayElem { array_id, index } => {
                let len = self.array_heap[array_id].len();
                if index >= len {
                    self.runtime_error(&format!(
                        "array lvalue read out of bounds: index {index}, length {len}"
                    ));
                }
                self.array_heap[array_id][index].clone()
            }
            LValue::StructField { struct_id, field } => self.heap[struct_id]
                .fields
                .get(&field)
                .cloned()
                .unwrap_or_else(|| self.runtime_error(&format!("missing struct field `{field}`"))),
        }
    }

    pub(crate) fn force_to_storable(&mut self, v: Type) -> Type {
        match v {
            Type::LValue(lv) => {
                let l = self.read_lvalue(lv);
                self.force_to_storable(l)
            }

            Type::LazyValue(_, _) => v, // keep relationships attached to locations
            other => other,
        }
    }

    pub(crate) fn exec_array_lvalue(&mut self) {
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
                    other => self
                        .runtime_error(&format!("indexing non-array (found {:?})", other)),
                }
            }

            Type::LValue(LValue::StructField { struct_id, field }) => {
                let field_val = self.heap[struct_id]
                    .fields
                    .get(&field)
                    .cloned()
                    .unwrap_or_else(|| self.runtime_error(&format!("missing struct field `{field}`")));

                let arr_val = self.force(field_val);
                match arr_val {
                    Type::ArrayRef(array_id) => {
                        self.stack.push(Type::LValue(LValue::ArrayElem {
                            array_id,
                            index: idx,
                        }));
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

    pub(crate) fn store_through_immutable(&mut self) {
        let value = self.pop();
        let target = self.pop();
        let stored = self.force_to_storable(value);

        match target {
            Type::LValue(LValue::StructField { struct_id, field }) => {
                let inst = &mut self.heap[struct_id];

                match inst.fields.get(&field) {
                    Some(Type::Uninitialized) => {}
                    Some(_) => self.runtime_error(&format!(
                        "cannot reassign immutable field `{}`",
                        field
                    )),
                    None => self.runtime_error(&format!("unknown struct field `{}`", field)),
                }

                inst.fields.insert(field.clone(), stored);
                inst.immutables.insert(field);
            }

            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                let imm = &mut self.array_immutables[array_id];

                if imm.contains(&index) {
                    self.runtime_error("cannot reassign immutable array element");
                }

                self.array_heap[array_id][index] = stored;
                imm.insert(index);
            }

            _ => self.runtime_error("immutable assignment only allowed on lvalues"),
        }
    }

    // =========================================================
    // Structs
    // =========================================================

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
                    .unwrap_or_else(|| self.runtime_error(&format!("missing struct field `{field}`")));

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

    pub(crate) fn instantiate_struct(
        &mut self,
        fields: Vec<(String, Option<CompiledStructFieldInit>)>,
    ) -> Type {
        let mut map = HashMap::new();
        let mut imm = HashSet::new();

        // Initialize all declared fields
        for (name, init) in &fields {
            match init {
                Some(CompiledStructFieldInit::Immutable(_)) => {
                    // immutable-with-initializer: the initializer will run later, but we want the slot
                    // to exist and be considered immutable from the start.
                    imm.insert(name.clone());
                    map.insert(name.clone(), Type::Uninitialized);
                }
                Some(CompiledStructFieldInit::Reactive(_)) => {
                    // reactive initializer stored later, slot exists now
                    map.insert(name.clone(), Type::Uninitialized);
                }
                Some(CompiledStructFieldInit::Mutable(_)) => {
                    // will be initialized later
                    map.insert(name.clone(), Type::Uninitialized);
                }
                None => {
                    // bare x starts uninitialized, so x := ... can be a one-time init
                    map.insert(name.clone(), Type::Uninitialized);
                }
            }
        }

        let id = self.heap.len();
        self.heap.push(StructInstance {
            fields: map,
            immutables: imm.clone(),
        });

        // Apply initializers (mutable/immutable are eager, reactive stores relationship)
        for (name, init) in fields {
            if let Some(init) = init {
                let value = match init {
                    CompiledStructFieldInit::Mutable(code)
                    | CompiledStructFieldInit::Immutable(code) => self.eval_struct_code(id, code),
                    CompiledStructFieldInit::Reactive(expr) => {
                        Type::LazyValue(expr, HashMap::new())
                    }
                };

                let stored = self.force_to_storable(value);
                let cloned = self.clone_value(stored);
                self.heap[id].fields.insert(name, cloned);
            }
        }

        Type::StructRef(id)
    }

    pub(crate) fn eval_struct_code(&mut self, struct_id: usize, code: Vec<Instruction>) -> Type {
        // Each evaluation creates a fresh immutable frame and binds all fields as LValues.
        self.immutable_stack.push(HashMap::new());

        {
            let scope = match self.immutable_stack.last_mut() {
                Some(scope) => scope,
                None => self.runtime_error("internal error: no immutable scope for struct eval"),
            };
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

        let result = self.run_reactive_code(code);
        self.immutable_stack.pop();
        result
    }

    pub(crate) fn eval_reactive_field_in_struct(
        &mut self,
        struct_id: usize,
        expr: &ReactiveExpr,
    ) -> Type {
        self.immutable_stack.push(HashMap::new());

        {
            let scope = match self.immutable_stack.last_mut() {
                Some(scope) => scope,
                None => self.runtime_error("internal error: no immutable scope for struct eval"),
            };
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

        let result = self.run_reactive_code(expr.code.clone());
        self.immutable_stack.pop();
        result
    }

    pub(crate) fn clone_value(&mut self, v: Type) -> Type {
        match v {
            Type::ArrayRef(id) => {
                let new_id = self.array_heap.len();
                self.array_heap.push(self.array_heap[id].clone());
                self.array_immutables
                    .push(self.array_immutables[id].clone());
                Type::ArrayRef(new_id)
            }

            Type::StructRef(id) => {
                let inst = self.heap[id].clone();
                let new_id = self.heap.len();
                self.heap.push(inst);
                Type::StructRef(new_id)
            }
            Type::LazyValue(expr, captured) => Type::LazyValue(expr, captured),
            Type::Integer(n) => Type::Integer(n),
            Type::Function { params, code } => Type::Function { params, code },
            Type::NativeFunction(name) => Type::NativeFunction(name),
            Type::LValue(_) => self.runtime_error("cannot clone lvalue"),
            Type::Char(c) => Type::Char(c),
            Type::Uninitialized => Type::Uninitialized,
        }
    }
}
