use super::VM;
use crate::grammar::{AST, LValue, StructFieldInit, StructInstance, Type};
use std::collections::{HashMap, HashSet};

impl VM {
    // =========================================================
    // Stack helpers
    // =========================================================

    pub(crate) fn pop(&mut self) -> Type {
        self.stack.pop().expect("stack underflow")
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
            other => panic!("type error: cannot coerce {:?} to int", other),
        }
    }

    pub(crate) fn as_usize_nonneg(&mut self, v: Type, what: &str) -> usize {
        let i = self.as_int(v);
        if i < 0 {
            panic!("{what} out of bounds: {i} is negative");
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
            other => panic!("cannot print value {:?}", other),
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
                    panic!("array index out of bounds: index {idx}, length {len}");
                }
                let elem = self.array_heap[id][idx].clone();
                let f = self.force(elem);
                self.stack.push(f);
            }
            other => panic!("type error: attempted to index non-array value {:?}", other),
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

    pub(crate) fn exec_store_index_reactive(&mut self, name: String, ast: Box<AST>) {
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
                    panic!("reactive array assignment out of bounds: index {idx}, length {len}");
                }
                self.array_heap[id][idx] = Type::LazyValue(frozen, captured);
            }
            other => panic!("type error: StoreIndexReactive on non-array {:?}", other),
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
                    other => panic!("FieldLValue on non-struct array element {:?}", other),
                }
            }

            other => panic!("invalid FieldLValue base {:?}", other),
        }
    }

    pub(crate) fn exec_store_through(&mut self) {
        let value = self.pop();
        let target = self.pop();

        let stored = self.force_to_storable(value);

        match target {
            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                if self.array_immutables[array_id].contains(&index) {
                    panic!("cannot reassign immutable array element");
                }

                let len = self.array_heap[array_id].len();
                if index >= len {
                    panic!("array assignment out of bounds");
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

    pub(crate) fn exec_store_through_reactive(&mut self, ast: Box<AST>) {
        let target = self.pop();

        let frozen = self.freeze_ast(ast);
        let captured = self.capture_immutables_for_ast(&frozen);

        match target {
            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                if self.array_immutables[array_id].contains(&index) {
                    panic!("cannot reassign immutable array element");
                }

                let len = self.array_heap[array_id].len();
                if index >= len {
                    panic!("reactive array assignment out of bounds");
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

    pub(crate) fn store_through_immutable(&mut self) {
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

            Type::LValue(LValue::ArrayElem { array_id, index }) => {
                let imm = &mut self.array_immutables[array_id];

                if imm.contains(&index) {
                    panic!("cannot reassign immutable array element");
                }

                self.array_heap[array_id][index] = stored;
                imm.insert(index);
            }

            _ => panic!("immutable assignment only allowed on lvalues"),
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

    pub(crate) fn exec_field_set(&mut self, field: String) {
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

    pub(crate) fn exec_field_set_reactive(&mut self, field: String, ast: Box<AST>) {
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

    pub(crate) fn instantiate_struct(
        &mut self,
        fields: Vec<(String, Option<StructFieldInit>)>,
    ) -> Type {
        let mut map = HashMap::new();
        let mut imm = HashSet::new();

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
                    // reactive initializer stored later, slot exists now
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
        let saved_local = self.local_env.take();
        let saved_immutables = std::mem::take(&mut self.immutable_stack);
        let saved_globals = std::mem::take(&mut self.global_env);

        self.local_env = None;
        self.immutable_stack = vec![HashMap::new()];
        self.global_env = HashMap::new();

        // Apply initializers (mutable/immutable are eager, reactive stores relationship)
        for (name, init) in fields {
            if let Some(init) = init {
                let value = match init {
                    StructFieldInit::Mutable(ast) | StructFieldInit::Immutable(ast) => {
                        self.eval_reactive_field_in_struct(id, ast)
                    }
                    StructFieldInit::Reactive(ast) => {
                        let frozen = Box::new(ast);
                        Type::LazyValue(frozen, HashMap::new())
                    }
                };

                let stored = self.force_to_storable(value);
                let cloned = self.clone_value(stored);
                self.heap[id].fields.insert(name, cloned);
            }
        }
        self.local_env = saved_local;
        self.immutable_stack = saved_immutables;
        self.global_env = saved_globals;

        Type::StructRef(id)
    }

    pub(crate) fn eval_reactive_field_in_struct(&mut self, struct_id: usize, ast: AST) -> Type {
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
            Type::LazyValue(ast, captured) => Type::LazyValue(ast, captured),
            Type::Integer(n) => Type::Integer(n),
            Type::Function { params, body } => Type::Function { params, body },
            Type::LValue(_) => panic!("cannot clone lvalue"),
            Type::Char(c) => Type::Char(c),
            Type::Uninitialized => Type::Uninitialized,
        }
    }
}
