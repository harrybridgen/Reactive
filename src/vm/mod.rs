pub mod call;
pub mod env;
pub mod exec;
pub mod reactive;
pub mod runtime;

use crate::grammar::{CompiledStructFieldInit, Instruction, StructInstance, Type};
use std::collections::{HashMap, HashSet};
struct CallFrame {
    code: Vec<Instruction>,
    labels: HashMap<String, usize>,
    pointer: usize,

    local_env: Option<HashMap<String, Type>>,
    immutable_stack: Vec<HashMap<String, Type>>,

    stack_base: usize,
    function_name: String,
}
pub struct VM {
    // Operand stack
    stack: Vec<Type>,

    // Global mutable environment (top-level only)
    global_env: HashMap<String, Type>,

    // Local mutable environment (function scope)
    local_env: Option<HashMap<String, Type>>,

    // Immutable scopes (:= bindings, function parameters, reactive captures)
    immutable_stack: Vec<HashMap<String, Type>>,

    // Bytecode execution state
    pointer: usize,
    code: Vec<Instruction>,
    labels: HashMap<String, usize>,

    // Runtime heaps
    struct_defs: HashMap<String, Vec<(String, Option<CompiledStructFieldInit>)>>,
    heap: Vec<StructInstance>,
    array_heap: Vec<Vec<Type>>,
    array_immutables: Vec<HashSet<usize>>,

    // Module import memoization
    imported_modules: HashSet<String>,

    // call stack
    call_stack: Vec<CallFrame>,
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
            array_immutables: Vec::new(),
            imported_modules: HashSet::new(),
            call_stack: Vec::new(),
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

    pub(crate) fn runtime_error(&self, message: &str) -> ! {
        println!("Runtime error: {message}");
        println!("Stack trace (most recent call last):");
        for frame in self.call_stack.iter().rev() {
            println!("  at {}()", frame.function_name);
        }
        std::process::exit(1);
    }
}
