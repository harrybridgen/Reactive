use crate::grammar::{
    AST, CompiledStructFieldInit, FieldAssignKind, Instruction, Operator, ReactiveExpr,
    StructFieldInit,
};
use std::collections::HashSet;
pub fn compile(
    ast: AST,
    code: &mut Vec<Instruction>,
    labels: &mut LabelGenerator,
    break_stack: &mut Vec<String>,
    continue_stack: &mut Vec<String>,
) {
    match ast {
        // ---------- literals ----------
        AST::Number(n) => code.push(Instruction::Push(n)),
        AST::Char(c) => code.push(Instruction::PushChar(c)),
        AST::Var(name) => code.push(Instruction::Load(name)),

        AST::StringLiteral(s) => compile_string_literal(s, code, labels),

        // ---------- expressions ----------
        AST::ArrayNew(size) => {
            compile(*size, code, labels, break_stack, continue_stack);
            code.push(Instruction::ArrayNew);
        }

        AST::Index(base, index) => {
            compile(*base, code, labels, break_stack, continue_stack);
            compile(*index, code, labels, break_stack, continue_stack);
            code.push(Instruction::ArrayGet);
        }

        AST::FieldAccess(base, field) => {
            compile(*base, code, labels, break_stack, continue_stack);
            code.push(Instruction::FieldGet(field));
        }

        AST::Operation(l, op, r) => {
            compile(*l, code, labels, break_stack, continue_stack);
            compile(*r, code, labels, break_stack, continue_stack);
            emit_operator(op, code);
        }

        AST::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            compile(*cond, code, labels, break_stack, continue_stack);

            let else_lbl = labels.fresh("ternary_else");
            let end_lbl = labels.fresh("ternary_end");

            code.push(Instruction::JumpIfZero(else_lbl.clone()));
            compile(*then_expr, code, labels, break_stack, continue_stack);
            code.push(Instruction::Jump(end_lbl.clone()));

            code.push(Instruction::Label(else_lbl));
            compile(*else_expr, code, labels, break_stack, continue_stack);

            code.push(Instruction::Label(end_lbl));
        }

        AST::Call { name, args } => {
            let argc = args.len();
            for a in args {
                compile(a, code, labels, break_stack, continue_stack);
            }
            code.push(Instruction::Call(name, argc));
        }

        // ---------- assignments ----------
        AST::Assign(name, expr) => {
            compile(*expr, code, labels, break_stack, continue_stack);
            code.push(Instruction::Store(name));
        }

        AST::ImmutableAssign(name, expr) => {
            compile(*expr, code, labels, break_stack, continue_stack);
            code.push(Instruction::StoreImmutable(name));
        }

        AST::ReactiveAssign(name, expr) => {
            let reactive = compile_reactive_expr(*expr);
            code.push(Instruction::StoreReactive(name, reactive));
        }

        AST::AssignTarget(target, value) => {
            compile_lvalue(*target, code, labels, break_stack, continue_stack);
            compile(*value, code, labels, break_stack, continue_stack);
            code.push(Instruction::StoreThrough);
        }

        AST::ReactiveAssignTarget(target, value) => {
            compile_lvalue(*target, code, labels, break_stack, continue_stack);
            let reactive = compile_reactive_expr(*value);
            code.push(Instruction::StoreThroughReactive(reactive));
        }

        AST::FieldAssign {
            base,
            field,
            value,
            kind,
        } => match kind {
            FieldAssignKind::Normal => {
                compile(*base, code, labels, break_stack, continue_stack);
                compile(*value, code, labels, break_stack, continue_stack);
                code.push(Instruction::FieldSet(field));
            }
            FieldAssignKind::Reactive => {
                compile(*base, code, labels, break_stack, continue_stack);
                let reactive = compile_reactive_expr(*value);
                code.push(Instruction::FieldSetReactive(field, reactive));
            }
            FieldAssignKind::Immutable => {
                panic!("immutable field assignment not allowed");
            }
        },

        // ---------- control ----------
        AST::IfElse(cond, then_block, else_block) => {
            compile(*cond, code, labels, break_stack, continue_stack);

            let else_lbl = labels.fresh("else");
            let end_lbl = labels.fresh("ifend");

            code.push(Instruction::JumpIfZero(else_lbl.clone()));

            // THEN block scope
            code.push(Instruction::PushImmutableContext);
            for s in then_block {
                compile(s, code, labels, break_stack, continue_stack);
            }
            code.push(Instruction::PopImmutableContext);

            code.push(Instruction::Jump(end_lbl.clone()));

            code.push(Instruction::Label(else_lbl));

            // ELSE block scope
            code.push(Instruction::PushImmutableContext);
            for s in else_block {
                compile(s, code, labels, break_stack, continue_stack);
            }
            code.push(Instruction::PopImmutableContext);

            code.push(Instruction::Label(end_lbl));
        }

        AST::Loop(body) => {
            let start = labels.fresh("loop_start");
            let end = labels.fresh("loop_end");
            break_stack.push(end.clone());
            continue_stack.push(start.clone());

            code.push(Instruction::PushImmutableContext);
            code.push(Instruction::Label(start.clone()));
            code.push(Instruction::ClearImmutableContext);

            for s in body {
                compile(s, code, labels, break_stack, continue_stack);
            }

            code.push(Instruction::Jump(start));
            code.push(Instruction::Label(end));
            code.push(Instruction::PopImmutableContext);

            break_stack.pop();
            continue_stack.pop();
        }

        AST::Break => {
            let target = break_stack
                .last()
                .expect("break used outside of loop")
                .clone();
            code.push(Instruction::Jump(target));
        }

        AST::Continue => {
            let target = continue_stack
                .last()
                .expect("continue used outside of loop")
                .clone();
            code.push(Instruction::Jump(target));
        }

        AST::Return(expr) => {
            if let Some(e) = expr {
                compile(*e, code, labels, break_stack, continue_stack);
            } else {
                code.push(Instruction::Push(0));
            }
            code.push(Instruction::Return);
        }

        // ---------- definitions ----------
        AST::FuncDef { name, params, body } => {
            let func_code = compile_function_body(body);
            code.push(Instruction::StoreFunction(name, params, func_code));
        }

        AST::StructDef { name, fields } => {
            let compiled_fields = compile_struct_fields(fields);
            code.push(Instruction::StoreStruct(name, compiled_fields));
        }

        AST::StructNew(name) => {
            code.push(Instruction::NewStruct(name));
        }

        AST::Import(path) => {
            code.push(Instruction::Import(path));
        }

        AST::Program(stmts) => {
            let mut has_main = false;

            for s in stmts {
                if let AST::FuncDef { name, .. } = &s {
                    if name == "main" {
                        has_main = true;
                    }
                }
                compile(s, code, labels, break_stack, continue_stack);
            }

            if !has_main {
                panic!("no `main` function defined");
            }

            code.push(Instruction::Call("main".to_string(), 0));
            code.push(Instruction::Return);
        }

        AST::Print(e) => {
            compile(*e, code, labels, break_stack, continue_stack);
            code.push(Instruction::Print);
        }

        AST::Println(e) => {
            compile(*e, code, labels, break_stack, continue_stack);
            code.push(Instruction::Println);
        }

        AST::ImmutableAssignTarget(target, value) => {
            compile_lvalue(*target, code, labels, break_stack, continue_stack);
            compile(*value, code, labels, break_stack, continue_stack);
            code.push(Instruction::StoreThroughImmutable);
        }
        AST::Cast { target, expr } => {
            compile(*expr, code, labels, break_stack, continue_stack);
            code.push(Instruction::Cast(target));
        }
    }
}

fn compile_function_body(body: Vec<AST>) -> Vec<Instruction> {
    let mut code = Vec::new();
    let mut labels = LabelGenerator::new();
    let mut break_stack = Vec::new();
    let mut continue_stack = Vec::new();

    for stmt in body {
        compile(
            stmt,
            &mut code,
            &mut labels,
            &mut break_stack,
            &mut continue_stack,
        );
    }

    code.push(Instruction::Return);
    code
}

fn compile_struct_fields(
    fields: Vec<(String, Option<StructFieldInit>)>,
) -> Vec<(String, Option<CompiledStructFieldInit>)> {
    fields
        .into_iter()
        .map(|(name, init)| {
            let compiled_init = match init {
                Some(StructFieldInit::Mutable(ast)) => {
                    Some(CompiledStructFieldInit::Mutable(compile_expr_to_code(ast)))
                }
                Some(StructFieldInit::Immutable(ast)) => Some(CompiledStructFieldInit::Immutable(
                    compile_expr_to_code(ast),
                )),
                Some(StructFieldInit::Reactive(ast)) => Some(CompiledStructFieldInit::Reactive(
                    compile_reactive_expr(ast),
                )),
                None => None,
            };
            (name, compiled_init)
        })
        .collect()
}

fn compile_expr_to_code(ast: AST) -> Vec<Instruction> {
    let mut code = Vec::new();
    let mut labels = LabelGenerator::new();
    let mut break_stack = Vec::new();
    let mut continue_stack = Vec::new();

    compile(
        ast,
        &mut code,
        &mut labels,
        &mut break_stack,
        &mut continue_stack,
    );
    code.push(Instruction::Return);
    code
}
fn compile_reactive_expr(ast: AST) -> ReactiveExpr {
    let mut names = HashSet::new();
    collect_free_vars(&ast, &mut names);

    let mut captures: Vec<String> = names.into_iter().collect();
    captures.sort();

    let code = compile_expr_to_code(ast);

    ReactiveExpr { code, captures }
}

fn collect_free_vars(ast: &AST, out: &mut HashSet<String>) {
    match ast {
        AST::Var(n) => {
            out.insert(n.clone());
        }
        AST::Operation(l, _, r) => {
            collect_free_vars(l, out);
            collect_free_vars(r, out);
        }
        AST::Index(b, i) => {
            collect_free_vars(b, out);
            collect_free_vars(i, out);
        }
        AST::FieldAccess(b, _) => {
            collect_free_vars(b, out);
        }
        AST::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_free_vars(cond, out);
            collect_free_vars(then_expr, out);
            collect_free_vars(else_expr, out);
        }
        AST::Call { args, .. } => {
            for a in args {
                collect_free_vars(a, out);
            }
        }
        AST::ArrayNew(size) => collect_free_vars(size, out),
        AST::Assign(_, rhs)
        | AST::ImmutableAssign(_, rhs)
        | AST::ReactiveAssign(_, rhs)
        | AST::ImmutableAssignTarget(_, rhs) => collect_free_vars(rhs, out),
        AST::AssignTarget(target, value) | AST::ReactiveAssignTarget(target, value) => {
            collect_free_vars(target, out);
            collect_free_vars(value, out);
        }
        AST::FieldAssign { base, value, .. } => {
            collect_free_vars(base, out);
            collect_free_vars(value, out);
        }
        AST::Cast { expr, .. } => collect_free_vars(expr, out),
        AST::Number(_)
        | AST::Char(_)
        | AST::StringLiteral(_)
        | AST::Program(_)
        | AST::IfElse(_, _, _)
        | AST::Loop(_)
        | AST::Break
        | AST::Continue
        | AST::Return(_)
        | AST::Print(_)
        | AST::Println(_)
        | AST::FuncDef { .. }
        | AST::StructDef { .. }
        | AST::StructNew(_)
        | AST::Import(_) => {}
    }
}

pub fn compile_module(
    ast: AST,
    code: &mut Vec<Instruction>,
    labels: &mut LabelGenerator,
    break_stack: &mut Vec<String>,
    continue_stack: &mut Vec<String>,
) {
    match ast {
        AST::Program(stmts) => {
            for s in stmts {
                compile(s, code, labels, break_stack, continue_stack);
            }
        }
        other => compile(other, code, labels, break_stack, continue_stack),
    }
}
fn compile_lvalue(
    ast: AST,
    code: &mut Vec<Instruction>,
    labels: &mut LabelGenerator,
    break_stack: &mut Vec<String>,
    continue_stack: &mut Vec<String>,
) {
    match ast {
        AST::Var(name) => {
            code.push(Instruction::Load(name));
        }

        AST::Index(base, index) => {
            compile_lvalue(*base, code, labels, break_stack, continue_stack);
            compile(*index, code, labels, break_stack, continue_stack);
            code.push(Instruction::ArrayLValue);
        }

        AST::FieldAccess(base, field) => {
            compile_lvalue(*base, code, labels, break_stack, continue_stack);
            code.push(Instruction::FieldLValue(field));
        }

        other => panic!("invalid assignment target: {:?}", other),
    }
}

fn emit_operator(op: Operator, code: &mut Vec<Instruction>) {
    use Operator::*;
    match op {
        Addition => code.push(Instruction::Add),
        Subtraction => code.push(Instruction::Sub),
        Multiplication => code.push(Instruction::Mul),
        Division => code.push(Instruction::Div),
        Modulo => code.push(Instruction::Modulo),
        Greater => code.push(Instruction::Greater),
        Less => code.push(Instruction::Less),
        Equal => code.push(Instruction::Equal),
        NotEqual => code.push(Instruction::NotEqual),
        GreaterEqual => code.push(Instruction::GreaterEqual),
        LessEqual => code.push(Instruction::LessEqual),
        And => code.push(Instruction::And),
        Or => code.push(Instruction::Or),
    }
}

fn compile_string_literal(s: String, code: &mut Vec<Instruction>, labels: &mut LabelGenerator) {
    code.push(Instruction::Push(s.chars().count() as i32));
    code.push(Instruction::ArrayNew);

    let tmp = labels.fresh("__strlit");
    code.push(Instruction::Store(tmp.clone()));

    for (i, ch) in s.chars().enumerate() {
        code.push(Instruction::Load(tmp.clone()));
        code.push(Instruction::Push(i as i32));
        code.push(Instruction::ArrayLValue);
        code.push(Instruction::PushChar(ch as u32));
        code.push(Instruction::StoreThrough);
    }

    code.push(Instruction::Load(tmp));
}

pub struct LabelGenerator {
    counter: usize,
}

impl LabelGenerator {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn fresh(&mut self, prefix: &str) -> String {
        let s = format!("{prefix}_{}", self.counter);
        self.counter += 1;
        s
    }
}
