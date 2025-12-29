use super::VM;
use crate::grammar::{Instruction, LValue, ReactiveExpr, Type};
use std::collections::HashMap;

impl VM {
    // =========================================================
    // Forcing / pull-based reactivity
    // =========================================================

    /// Forces a value for use (pull-based reactivity):
    /// - LazyValue is evaluated
    /// - LValue is dereferenced
    /// - Everything else is returned as-is
    pub(crate) fn force(&mut self, v: Type) -> Type {
        match v {
            Type::LazyValue(expr, captured) => {
                self.immutable_stack.push(captured);
                let out = self.evaluate_reactive_expr(&expr);
                self.immutable_stack.pop();
                self.force(out)
            }

            Type::LValue(lv) => match lv {
                LValue::StructField { struct_id, field } => {
                    let val = self.heap[struct_id]
                        .fields
                        .get(&field)
                        .cloned()
                        .unwrap_or_else(|| self.runtime_error(&format!("missing struct field `{}`", field)));

                    self.force_struct_field(struct_id, val)
                }

                LValue::ArrayElem { array_id, index } => {
                    let val = self.read_lvalue(LValue::ArrayElem { array_id, index });
                    self.force(val)
                }
                LValue::VecElem { vec_id, index } => {
                    let val = self.read_lvalue(LValue::VecElem { vec_id, index });
                    self.force(val)
                }
            },

            other => other,
        }
    }

    /// Like force, but when the LazyValue originates from a struct field, it evaluates
    /// with a struct-local immutable frame binding all fields as LValues.
    pub(crate) fn force_struct_field(&mut self, struct_id: usize, v: Type) -> Type {
        match v {
            Type::LazyValue(expr, captured) => {
                self.immutable_stack.push(captured);
                let out = self.eval_reactive_field_in_struct(struct_id, &expr);
                self.immutable_stack.pop();
                self.force(out)
            }
            other => self.force(other),
        }
    }

    // =========================================================
    // Reactive evaluation helpers
    // =========================================================

    pub(crate) fn evaluate_reactive_expr(&mut self, expr: &ReactiveExpr) -> Type {
        self.run_reactive_code(expr.code.clone())
    }

    pub(crate) fn capture_immutables(&self, names: &[String]) -> HashMap<String, Type> {
        let mut captured = HashMap::new();
        for n in names {
            if let Some(v) = self.find_immutable(n).cloned() {
                captured.insert(n.clone(), v);
            }
        }
        captured
    }

    pub(crate) fn run_reactive_code(&mut self, code: Vec<Instruction>) -> Type {
        let saved_code = std::mem::replace(&mut self.code, code);
        let saved_labels = std::mem::replace(&mut self.labels, Self::build_labels(&self.code));
        let saved_ptr = self.pointer;
        let saved_stack_len = self.stack.len();

        self.pointer = 0;
        self.run();

        let result = if self.stack.len() > saved_stack_len {
            self.pop()
        } else {
            Type::Integer(0)
        };

        self.code = saved_code;
        self.labels = saved_labels;
        self.pointer = saved_ptr;

        result
    }
}
