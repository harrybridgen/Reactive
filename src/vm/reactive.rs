use super::VM;
use crate::grammar::{AST, LValue, Operator, Type};
use std::collections::{HashMap, HashSet};

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
            Type::LazyValue(ast, captured) => {
                self.immutable_stack.push(captured);
                let out = self.eval_value(*ast);
                self.immutable_stack.pop();
                self.force(out)
            }

            Type::LValue(lv) => match lv {
                LValue::StructField { struct_id, field } => {
                    // IMPORTANT: struct reactive fields must be forced with struct-local bindings
                    let val = self.heap[struct_id]
                        .fields
                        .get(&field)
                        .cloned()
                        .unwrap_or_else(|| panic!("missing struct field `{}`", field));

                    self.force_struct_field(struct_id, val)
                }

                LValue::ArrayElem { array_id, index } => {
                    let val = self.read_lvalue(LValue::ArrayElem { array_id, index });
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
            Type::LazyValue(ast, captured) => {
                self.immutable_stack.push(captured);
                let out = self.eval_reactive_field_in_struct(struct_id, *ast);
                self.immutable_stack.pop();
                self.force(out)
            }
            other => self.force(other),
        }
    }

    // =========================================================
    // Lazy/reactive evaluation (AST interpreter)
    // =========================================================
    pub(crate) fn eval_value(&mut self, ast: AST) -> Type {
        match ast {
            AST::Number(n) => Type::Integer(n),
            AST::Char(c) => Type::Char(c),

            AST::Var(name) => {
                if let Some(v) = self.find_immutable(&name).cloned() {
                    return v;
                }
                if let Some(v) = self.lookup_var(&name).cloned() {
                    return v;
                }

                self.dbg_dump_state(&format!(
                    "UNBOUND VAR lookup failed for `{}` while eval_value(Var)",
                    name
                ));
                panic!("undefined variable: {name}");
            }

            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let value = self.eval_value(*cond);
                let c = self.as_int(value);
                if c != 0 {
                    self.eval_value(*then_expr)
                } else {
                    self.eval_value(*else_expr)
                }
            }

            AST::ArrayNew(size_ast) => {
                let value = self.eval_value(*size_ast);
                let n = self.as_usize_nonneg(value, "array size");
                let id = self.array_heap.len();
                self.array_heap.push(vec![Type::Integer(0); n]);
                self.array_immutables.push(HashSet::new());
                Type::ArrayRef(id)
            }

            AST::StringLiteral(s) => {
                let id = self.array_heap.len();
                let mut arr = Vec::with_capacity(s.chars().count());
                for c in s.chars() {
                    arr.push(Type::Char(c as u32));
                }
                self.array_heap.push(arr);
                Type::ArrayRef(id)
            }

            AST::FieldAccess(base, field) => {
                let value = self.eval_value(*base);
                let obj = self.force(value);
                match obj {
                    Type::StructRef(id) => {
                        let v = self
                            .heap
                            .get(id)
                            .and_then(|s| s.fields.get(&field))
                            .cloned()
                            .unwrap_or_else(|| panic!("missing struct field `{field}`"));
                        self.force_struct_field(id, v)
                    }
                    other => panic!("type error: field access on non-struct {:?}", other),
                }
            }

            AST::Index(base, index) => {
                let idx_val = self.eval_value(*index);
                let idx = self.as_usize_nonneg(idx_val, "array index");

                let base_val = self.eval_value(*base);
                let arr = self.force(base_val);

                match arr {
                    Type::ArrayRef(id) => {
                        let len = self.array_heap[id].len();
                        if idx >= len {
                            panic!("array index out of bounds: index {idx}, length {len}");
                        }
                        let elem = self.array_heap[id][idx].clone();
                        self.force(elem)
                    }
                    other => panic!("type error: indexing non-array {:?}", other),
                }
            }

            AST::Call { name, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_value(a));
                }
                let f = self
                    .global_env
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| panic!("call error: `{name}` is not defined"));

                self.call_function(f, vals)
            }

            AST::Operation(l, op, r) => {
                let lv = self.eval_value(*l);
                let rv = self.eval_value(*r);

                match (&op, self.force(lv), self.force(rv)) {
                    (Operator::Addition, Type::Char(c), Type::Integer(n))
                    | (Operator::Addition, Type::Integer(n), Type::Char(c)) => {
                        Type::Char((c as i32 + n) as u32)
                    }
                    (Operator::Subtraction, Type::Char(c), Type::Integer(n))
                    | (Operator::Subtraction, Type::Integer(n), Type::Char(c)) => {
                        Type::Char((c as i32 - n) as u32)
                    }
                    (Operator::Modulo, Type::Char(c), Type::Integer(n))
                    | (Operator::Modulo, Type::Integer(n), Type::Char(c)) => {
                        Type::Char((c as i32 % n) as u32)
                    }
                    (op, a, b) => {
                        let ai = self.as_int(a);
                        let bi = self.as_int(b);
                        Type::Integer(match op {
                            Operator::Addition => ai + bi,
                            Operator::Subtraction => ai - bi,
                            Operator::Multiplication => ai * bi,
                            Operator::Division => ai / bi,
                            Operator::Greater => (ai > bi) as i32,
                            Operator::Less => (ai < bi) as i32,
                            Operator::Equal => (ai == bi) as i32,
                            Operator::NotEqual => (ai != bi) as i32,
                            Operator::GreaterEqual => (ai >= bi) as i32,
                            Operator::LessEqual => (ai <= bi) as i32,
                            Operator::And => ((ai > 0) && (bi > 0)) as i32,
                            Operator::Or => ((ai > 0) || (bi > 0)) as i32,
                            Operator::Modulo => ai % bi,
                        })
                    }
                }
            }

            other => panic!("eval_value(): unsupported AST variant: {:?}", other),
        }
    }

    // =========================================================
    // Reactive capture utilities
    // =========================================================

    pub(crate) fn ast_free_vars(&self, ast: &AST, out: &mut HashSet<String>) {
        match ast {
            AST::Var(n) => {
                out.insert(n.clone());
            }
            AST::Operation(l, _, r) => {
                self.ast_free_vars(l, out);
                self.ast_free_vars(r, out);
            }
            AST::Index(b, i) => {
                self.ast_free_vars(b, out);
                self.ast_free_vars(i, out);
            }
            AST::FieldAccess(b, _) => {
                self.ast_free_vars(b, out);
            }
            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.ast_free_vars(cond, out);
                self.ast_free_vars(then_expr, out);
                self.ast_free_vars(else_expr, out);
            }
            AST::Call { args, .. } => {
                for a in args {
                    self.ast_free_vars(a, out);
                }
            }
            AST::Number(_) | AST::Char(_) | AST::StringLiteral(_) => {}
            _ => {}
        }
    }

    pub(crate) fn capture_immutables_for_ast(&self, ast: &AST) -> HashMap<String, Type> {
        let mut names = HashSet::new();
        self.ast_free_vars(ast, &mut names);

        let mut cap = HashMap::new();
        for n in names {
            if let Some(v) = self.find_immutable(&n).cloned() {
                cap.insert(n, v);
            }
        }
        cap
    }

    /// Freeze immutables that are integers by replacing Var(x) with Number(n) when x
    /// resolves to an immutable integer in the current immutable stack.
    pub(crate) fn freeze_ast(&self, ast: Box<AST>) -> Box<AST> {
        match *ast {
            AST::Var(name) => {
                if let Some(Type::Integer(n)) = self.find_immutable(&name) {
                    Box::new(AST::Number(*n))
                } else {
                    Box::new(AST::Var(name))
                }
            }
            AST::Number(n) => Box::new(AST::Number(n)),
            AST::Char(c) => Box::new(AST::Char(c)),
            AST::Operation(l, o, r) => {
                Box::new(AST::Operation(self.freeze_ast(l), o, self.freeze_ast(r)))
            }
            AST::Index(b, i) => Box::new(AST::Index(self.freeze_ast(b), self.freeze_ast(i))),
            AST::FieldAccess(b, f) => Box::new(AST::FieldAccess(self.freeze_ast(b), f)),
            AST::Ternary {
                cond,
                then_expr,
                else_expr,
            } => Box::new(AST::Ternary {
                cond: self.freeze_ast(cond),
                then_expr: self.freeze_ast(then_expr),
                else_expr: self.freeze_ast(else_expr),
            }),
            other => Box::new(other),
        }
    }
}
