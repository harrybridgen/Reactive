use super::VM;
use crate::{
    grammar::{Instruction, Type},
    vm::CallFrame,
};
use std::collections::HashMap;

impl VM {
    // =========================================================
    // Instruction entry point
    // =========================================================
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

    // =========================================================
    // Function execution
    // =========================================================
    pub(crate) fn call_function(&mut self, name: String, f: Type, args: Vec<Type>) -> Type {
        match f {
            Type::Function { params, code } => {
                // Build immutable stack: global + params
                let global_immutables = self.immutable_stack[0].clone();
                let mut imm_stack = vec![global_immutables, HashMap::new()];

                {
                    let scope = imm_stack.last_mut().unwrap();
                    for (p, v) in params.into_iter().zip(args) {
                        scope.insert(p, v);
                    }
                }

                let local_env = Some(HashMap::new());

                let labels = Self::build_labels(&code);

                // Push call frame
                self.push_frame(name, code, labels, local_env, imm_stack);

                // Execute
                self.run();

                // Pop frame and return value
                self.pop_frame()
            }
            _ => self.runtime_error("attempted to call non-function"),
        }
    }

    fn call_native(&mut self, name: String, args: Vec<Type>) -> Type {
        let f = self
            .native_functions
            .get(&name)
            .copied()
            .unwrap_or_else(|| {
                self.runtime_error(&format!(
                    "call error: native function `{}` is not registered",
                    name
                ))
            });

        self.push_native_frame(name);
        let result = f(self, args);
        if self.call_stack.pop().is_none() {
            self.runtime_error("call stack underflow after native call");
        }
        result
    }

    fn push_frame(
        &mut self,
        function_name: String,
        code: Vec<Instruction>,
        labels: HashMap<String, usize>,
        local_env: Option<HashMap<String, Type>>,
        immutable_stack: Vec<HashMap<String, Type>>,
    ) {
        let frame = CallFrame {
            code: std::mem::replace(&mut self.code, code),
            labels: std::mem::replace(&mut self.labels, labels),
            pointer: self.pointer,

            local_env: std::mem::replace(&mut self.local_env, local_env),
            immutable_stack: std::mem::replace(&mut self.immutable_stack, immutable_stack),

            stack_base: self.stack.len(),
            function_name,
        };

        self.pointer = 0;
        self.call_stack.push(frame);
    }

    fn push_native_frame(&mut self, function_name: String) {
        let frame = CallFrame {
            code: Vec::new(),
            labels: HashMap::new(),
            pointer: 0,
            local_env: None,
            immutable_stack: Vec::new(),
            stack_base: self.stack.len(),
            function_name,
        };
        self.call_stack.push(frame);
    }

    fn pop_frame(&mut self) -> Type {
        let frame = match self.call_stack.pop() {
            Some(frame) => frame,
            None => self.runtime_error("call stack underflow"),
        };

        let ret = if self.stack.len() > frame.stack_base {
            self.stack.pop().unwrap()
        } else {
            Type::Integer(0)
        };

        self.code = frame.code;
        self.labels = frame.labels;
        self.pointer = frame.pointer;
        self.local_env = frame.local_env;
        self.immutable_stack = frame.immutable_stack;

        ret
    }

    // =========================================================
    // Module imports
    // =========================================================
    pub(crate) fn import_module(&mut self, path: Vec<String>) {
        if path.len() == 2 && path[0] == "std" && path[1] == "file" {
            self.install_native_fs();
        }
        if path.len() == 2 && path[0] == "std" && path[1] == "buf" {
            self.install_native_buf();
        }
        if path.len() == 2 && path[0] == "std" && path[1] == "vec" {
            self.install_native_vec();
        }
        if path.len() == 2 && path[0] == "std" && path[1] == "input" {
            self.install_native_input();
        }
    }
}
