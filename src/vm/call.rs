use super::VM;
use crate::grammar::{Instruction, Type};
use std::collections::HashMap;

impl VM {
    // =========================================================
    // Instruction entry point
    // =========================================================
    pub(crate) fn exec_call(&mut self, name: String, argc: usize) {
        let args = self.pop_args(argc);

        let f = self.global_env.get(&name).cloned().unwrap_or_else(|| {
            panic!(
                "call error: `{}` is not defined (attempted to call with {} argument(s))",
                name, argc
            )
        });

        let ret = match f {
            Type::Function { .. } => self.call_function(f, args),
            other => panic!(
                "call error: `{}` is not a function (found {:?})",
                name, other
            ),
        };

        self.stack.push(ret);
    }

    // =========================================================
    // Function execution
    // =========================================================
    pub(crate) fn call_function(&mut self, f: Type, args: Vec<Type>) -> Type {
        match f {
            Type::Function { params, body } => {
                // Save VM state
                let saved_local = self.local_env.take();
                // Save full caller immutable stack
                let saved_immutables = std::mem::take(&mut self.immutable_stack);

                // Keep only the global immutable frame for the callee
                let global = saved_immutables[0].clone();
                self.immutable_stack = vec![global];

                // Fresh param frame
                self.immutable_stack.push(HashMap::new());

                self.local_env = Some(HashMap::new());

                // Bind parameters as immutables
                {
                    let scope = self.immutable_stack.last_mut().unwrap();
                    for (p, v) in params.into_iter().zip(args) {
                        scope.insert(p, v);
                    }
                }

                // Compile function body
                let mut code = Vec::new();
                let mut lg = crate::compiler::LabelGenerator::new();
                let mut break_stack = Vec::new();

                for stmt in body {
                    crate::compiler::compile(stmt, &mut code, &mut lg, &mut break_stack);
                }
                code.push(Instruction::Return);

                // Swap execution context
                let saved_code = std::mem::replace(&mut self.code, code);
                let saved_labels =
                    std::mem::replace(&mut self.labels, Self::build_labels(&self.code));
                let saved_ptr = self.pointer;
                let saved_stack_len = self.stack.len();

                self.pointer = 0;
                self.run();

                // Retrieve return value
                let ret = if self.stack.len() > saved_stack_len {
                    self.pop()
                } else {
                    Type::Integer(0)
                };

                // Restore VM state
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

    // =========================================================
    // Module imports
    // =========================================================
    pub(crate) fn import_module(&mut self, path: Vec<String>) {
        let file_path = format!("project/{}.rx", path.join("/"));

        let source = std::fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("could not import module `{}`", file_path));

        let tokens = crate::tokenizer::tokenize(&source);
        let ast = crate::parser::parse(tokens);

        let mut code = Vec::new();
        let mut lg = crate::compiler::LabelGenerator::new();
        let mut break_stack = Vec::new();

        crate::compiler::compile_module(ast, &mut code, &mut lg, &mut break_stack);

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
