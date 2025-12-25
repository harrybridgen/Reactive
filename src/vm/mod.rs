pub mod call;
pub mod debug;
pub mod env;
pub mod exec;
pub mod reactive;
pub mod runtime;

use crate::grammar::{Instruction, StructFieldInit, StructInstance, Type};
use std::collections::{HashMap, HashSet};

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
    struct_defs: HashMap<String, Vec<(String, Option<StructFieldInit>)>>,
    heap: Vec<StructInstance>,
    array_heap: Vec<Vec<Type>>,
    array_immutables: Vec<HashSet<usize>>,

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
            array_immutables: Vec::new(),
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
}
