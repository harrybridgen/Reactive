use super::VM;
use crate::grammar::Type;

impl VM {
    // =========================================================
    // Debug helpers
    // =========================================================

    pub(crate) fn dbg_short_type(&self, v: &Type) -> String {
        match v {
            Type::Integer(n) => format!("Int({})", n),
            Type::Char(c) => format!("Char({})", c),
            Type::ArrayRef(id) => format!("ArrayRef({})", id),
            Type::StructRef(id) => format!("StructRef({})", id),
            Type::Function { params, .. } => format!("Function(params={:?})", params),
            Type::LValue(lv) => format!("LValue({:?})", lv),
            Type::LazyValue(ast, captured) => {
                format!("Lazy({:?}, cap={:?})", ast, captured.keys())
            }
            Type::Uninitialized => "Uninitialized".to_string(),
        }
    }

    pub(crate) fn dump_env_keys(&self) -> Vec<String> {
        let mut keys: Vec<_> = self.global_env.keys().cloned().collect();
        keys.sort();
        keys
    }

    pub(crate) fn dump_stack(&self) -> Vec<String> {
        self.stack.iter().map(|v| self.dbg_short_type(v)).collect()
    }

    pub(crate) fn dbg_dump_state(&self, headline: &str) {
        if !self.debug {
            return;
        }

        eprintln!("\n================ VM DEBUG ================");
        eprintln!("{}", headline);
        eprintln!(
            "ip={} instr={:?}",
            self.pointer,
            self.code.get(self.pointer)
        );
        eprintln!("reactive_ctx={:?}", self.debug_reactive_ctx);
        eprintln!("stack(len={}): {:?}", self.stack.len(), self.dump_stack());
        eprintln!("env keys: {:?}", self.dump_env_keys());
        eprintln!("immutable frames: {}", self.immutable_stack.len());

        for (frame_i, scope) in self.immutable_stack.iter().enumerate() {
            let mut keys: Vec<_> = scope.keys().cloned().collect();
            keys.sort();
            eprintln!("  frame[{frame_i}] keys={keys:?}");
        }

        eprintln!("heap structs: {}", self.heap.len());
        eprintln!("array heap: {}", self.array_heap.len());
        eprintln!("==========================================\n");
    }
}
