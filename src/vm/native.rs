use super::{NativeFunction, VM};
use crate::grammar::Type;
use std::collections::HashSet;
use std::path::Path;

impl VM {
    pub(crate) fn install_native_fs(&mut self) {
        self.register_native("internal_file_read", native_read);
        self.register_native("internal_file_write", native_write);
        self.register_native("internal_file_exists", native_exists);
        self.register_native("internal_file_remove", native_remove);
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
