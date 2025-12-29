use super::{NativeFunction, VM};
use crate::grammar::Type;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

impl VM {
    pub(crate) fn install_native_fs(&mut self) {
        self.register_native("internal_file_read", native_read);
        self.register_native("internal_file_write", native_write);
        self.register_native("internal_file_exists", native_exists);
        self.register_native("internal_file_remove", native_remove);
    }

    pub(crate) fn install_native_buf(&mut self) {
        self.register_native("internal_buf_new", native_buf_new);
        self.register_native("internal_buf_push_char", native_buf_push_char);
        self.register_native("internal_buf_push_str", native_buf_push_str);
        self.register_native("internal_buf_to_string", native_buf_to_string);
        self.register_native("internal_buf_write_file", native_buf_write_file);
    }

    pub(crate) fn install_native_vec(&mut self) {
        self.register_native("internal_vec_new", native_vec_new);
        self.register_native("internal_vec_push", native_vec_push);
        self.register_native("internal_vec_pop", native_vec_pop);
    }

    fn register_native(&mut self, name: &str, f: NativeFunction) {
        self.native_functions.insert(name.to_string(), f);
        self.global_env
            .insert(name.to_string(), Type::NativeFunction(name.to_string()));
    }

    fn value_to_string(&mut self, v: Type, what: &str) -> String {
        match self.force(v) {
            Type::ArrayRef(id) => {
                let elems = self.array_heap[id].clone();
                let mut out = String::with_capacity(elems.len());
                for elem in elems {
                    match self.force(elem) {
                        Type::Char(c) => match char::from_u32(c) {
                            Some(ch) => out.push(ch),
                            None => self
                                .runtime_error(&format!("{what} contains invalid char code {c}")),
                        },
                        other => self.runtime_error(&format!(
                            "{what} must be a string (array of chars), found {:?}",
                            other
                        )),
                    }
                }
                out
            }
            Type::VecRef(id) => {
                let elems = self.vec_heap[id].clone();
                let mut out = String::with_capacity(elems.len());
                for elem in elems {
                    match self.force(elem) {
                        Type::Char(c) => match char::from_u32(c) {
                            Some(ch) => out.push(ch),
                            None => self
                                .runtime_error(&format!("{what} contains invalid char code {c}")),
                        },
                        other => self.runtime_error(&format!(
                            "{what} must be a string (array of chars), found {:?}",
                            other
                        )),
                    }
                }
                out
            }
            other => self.runtime_error(&format!(
                "{what} must be a string (array of chars), found {:?}",
                other
            )),
        }
    }

    fn string_to_array(&mut self, s: &str) -> Type {
        let id = self.array_heap.len();
        let elems: Vec<Type> = s.chars().map(|ch| Type::Char(ch as u32)).collect();
        self.array_heap.push(elems);
        self.array_immutables.push(HashSet::new());
        Type::ArrayRef(id)
    }
}

fn native_read(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_file_read expects 1 argument, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_read path");
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| {
            vm.runtime_error(&format!(
                "internal_file_read failed for `{}`: {}",
                path, e
            ))
        });
    vm.string_to_array(&contents)
}

fn native_write(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_file_write expects 2 arguments, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_write path");
    let contents = vm.value_to_string(args[1].clone(), "internal_file_write contents");

    std::fs::write(&path, contents.as_bytes())
        .unwrap_or_else(|e| {
            vm.runtime_error(&format!(
                "internal_file_write failed for `{}`: {}",
                path, e
            ))
        });

    let count = contents.chars().count();
    let count_i32 = i32::try_from(count)
        .unwrap_or_else(|_| vm.runtime_error("write contents too large for int"));
    Type::Integer(count_i32)
}

fn native_exists(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_file_exists expects 1 argument, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_exists path");
    let exists = Path::new(&path).exists();
    Type::Integer(if exists { 1 } else { 0 })
}

fn native_remove(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_file_remove expects 1 argument, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_remove path");
    std::fs::remove_file(&path)
        .unwrap_or_else(|e| {
            vm.runtime_error(&format!(
                "internal_file_remove failed for `{}`: {}",
                path, e
            ))
        });
    Type::Integer(1)
}

fn native_buf_new(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_buf_new expects 1 argument, got {}",
            args.len()
        ));
    }

    let cap = vm.as_usize_nonneg(args[0].clone(), "internal_buf_new capacity");
    let id = vm.buffer_heap.len();
    vm.buffer_heap.push(Vec::with_capacity(cap));
    Type::BufferRef(id)
}

fn native_buf_push_char(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_buf_push_char expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_push_char expects buffer, found {:?}",
            other
        )),
    };

    let ch = match vm.force(args[1].clone()) {
        Type::Char(c) => c,
        other => vm.runtime_error(&format!(
            "internal_buf_push_char expects char, found {:?}",
            other
        )),
    };

    vm.buffer_heap[id].push(ch);
    Type::BufferRef(id)
}

fn native_buf_push_str(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_buf_push_str expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_push_str expects buffer, found {:?}",
            other
        )),
    };

    let str_id = match vm.force(args[1].clone()) {
        Type::ArrayRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_push_str expects string, found {:?}",
            other
        )),
    };

    let elems = vm.array_heap[str_id].clone();
    for elem in elems {
        match vm.force(elem) {
            Type::Char(c) => vm.buffer_heap[id].push(c),
            other => vm.runtime_error(&format!(
                "internal_buf_push_str expects string of chars, found {:?}",
                other
            )),
        }
    }

    Type::BufferRef(id)
}

fn native_buf_to_string(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_buf_to_string expects 1 argument, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_to_string expects buffer, found {:?}",
            other
        )),
    };

    let elems: Vec<Type> = vm.buffer_heap[id]
        .iter()
        .map(|c| Type::Char(*c))
        .collect();
    let arr_id = vm.array_heap.len();
    vm.array_heap.push(elems);
    vm.array_immutables.push(HashSet::new());
    Type::ArrayRef(arr_id)
}

fn native_buf_write_file(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_buf_write_file expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_write_file expects buffer, found {:?}",
            other
        )),
    };
    let path = vm.value_to_string(args[1].clone(), "internal_buf_write_file path");

    let mut file = std::fs::File::create(&path).unwrap_or_else(|e| {
        vm.runtime_error(&format!(
            "internal_buf_write_file failed for `{}`: {}",
            path, e
        ))
    });

    let mut count = 0usize;
    for c in vm.buffer_heap[id].iter().copied() {
        let ch = char::from_u32(c)
            .unwrap_or_else(|| vm.runtime_error(&format!("invalid char code {c} in buffer")));
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        file.write_all(encoded.as_bytes()).unwrap_or_else(|e| {
            vm.runtime_error(&format!(
                "internal_buf_write_file failed for `{}`: {}",
                path, e
            ))
        });
        count += 1;
    }

    let count_i32 = i32::try_from(count)
        .unwrap_or_else(|_| vm.runtime_error("buffer too large for int"));
    Type::Integer(count_i32)
}

fn native_vec_new(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_vec_new expects 1 argument, got {}",
            args.len()
        ));
    }

    let cap = vm.as_usize_nonneg(args[0].clone(), "internal_vec_new capacity");
    let id = vm.vec_heap.len();
    vm.vec_heap.push(Vec::with_capacity(cap));
    vm.vec_immutables.push(HashSet::new());
    Type::VecRef(id)
}

fn native_vec_push(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_vec_push expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::VecRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_vec_push expects vec, found {:?}",
            other
        )),
    };

    let val = args[1].clone();
    vm.vec_heap[id].push(val);
    Type::VecRef(id)
}

fn native_vec_pop(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_vec_pop expects 1 argument, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::VecRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_vec_pop expects vec, found {:?}",
            other
        )),
    };

    let value = vm
        .vec_heap[id]
        .pop()
        .unwrap_or_else(|| vm.runtime_error("internal_vec_pop on empty vec"));
    let len = vm.vec_heap[id].len();
    vm.vec_immutables[id].remove(&len);
    value
}
