use crate::grammar::{AST, Instruction, Operator, FieldAssignKind};

pub fn compile(
    ast: AST,
    code: &mut Vec<Instruction>,
    label_gen: &mut LabelGenerator,
    break_stack: &mut Vec<String>,
) {
    match ast {
        AST::Number(n) => code.push(Instruction::Push(n)),
        AST::Var(name) => code.push(Instruction::Load(name)),

        AST::ArrayNew(size_expr) => {
            compile(*size_expr, code, label_gen, break_stack);
            code.push(Instruction::ArrayNew);
        }

        AST::Index(base, index) => {
            compile(*base, code, label_gen, break_stack);
            compile(*index, code, label_gen, break_stack);
            code.push(Instruction::ArrayGet);
        }

        AST::AssignIndex(name, index_expr, value_expr) => {
            compile(*index_expr, code, label_gen, break_stack);
            compile(*value_expr, code, label_gen, break_stack);
            code.push(Instruction::StoreIndex(name));
        }

        AST::ReactiveAssignIndex(name, index_expr, value_expr) => {
            compile(*index_expr, code, label_gen, break_stack);
            code.push(Instruction::StoreIndexReactive(name, value_expr));
        }

        AST::Operation(left, operator, right) => {
            compile(*left, code, label_gen, break_stack);
            compile(*right, code, label_gen, break_stack);
            match operator {
                Operator::Addition => code.push(Instruction::Add),
                Operator::Division => code.push(Instruction::Div),
                Operator::Multiplication => code.push(Instruction::Mul),
                Operator::Subtraction => code.push(Instruction::Sub),
                Operator::Greater => code.push(Instruction::Greater),
                Operator::Less => code.push(Instruction::Less),
                Operator::Equal => code.push(Instruction::Equal),
                Operator::Or => code.push(Instruction::Or),
                Operator::And => code.push(Instruction::And),
                Operator::GreaterEqual => code.push(Instruction::GreaterEqual),
                Operator::LessEqual => code.push(Instruction::LessEqual),
                Operator::NotEqual => code.push(Instruction::NotEqual),
            }
        }

        AST::Assign(name, expr) => {
            compile(*expr, code, label_gen, break_stack);
            code.push(Instruction::Store(name));
        }

        AST::ReactiveAssign(name, expr) => {
            code.push(Instruction::StoreReactive(name, expr));
        }

        AST::ImmutableAssign(name, expr) => {
            compile(*expr, code, label_gen, break_stack);
            code.push(Instruction::StoreImmutable(name));
        }

        AST::FieldAccess(base, field) => {
            compile(*base, code, label_gen, break_stack);
            code.push(Instruction::FieldGet(field));
        }
AST::FieldAssign { base, field, value, kind } => {
    match kind {
        FieldAssignKind::Normal => {
            compile(*base, code, label_gen, break_stack);
            compile(*value, code, label_gen, break_stack);
            code.push(Instruction::FieldSet(field));
        }

        FieldAssignKind::Reactive => {
            compile(*base, code, label_gen, break_stack);
            code.push(Instruction::FieldSetReactive(field, value));
        }

        FieldAssignKind::Immutable => {
            panic!("immutable field assignment not allowed");
        }
    }
}



        AST::FuncDef { name, params, body } => {
            code.push(Instruction::StoreFunction(name, params, body));
        }

        AST::Return(_) => {}

        AST::Call { name, args } => {
            let argc = args.len();
            for arg in args {
                compile(arg, code, label_gen, break_stack);
            }
            code.push(Instruction::Call(name, argc));
        }

        AST::StructDef { name, fields } => {
            code.push(Instruction::StoreStruct(name, fields));
        }

        AST::StructNew(name) => {
            code.push(Instruction::NewStruct(name));
        }

        AST::Print(expr) => {
            compile(*expr, code, label_gen, break_stack);
            code.push(Instruction::Print);
        }

        AST::Println(expr) => {
            compile(*expr, code, label_gen, break_stack);
            code.push(Instruction::Println);
        }

        AST::Program(statements) => {
            for stmt in statements {
                compile(stmt, code, label_gen, break_stack);
            }
        }

        AST::IfElse(cond, if_branch, else_branch) => {
            compile(*cond, code, label_gen, break_stack);
            let else_label = label_gen.fresh("else");
            let end_label = label_gen.fresh("end");
            code.push(Instruction::JumpIfZero(else_label.clone()));
            for stmt in if_branch {
                compile(stmt, code, label_gen, break_stack);
            }
            code.push(Instruction::Jump(end_label.clone()));
            code.push(Instruction::Label(else_label));
            for stmt in else_branch {
                compile(stmt, code, label_gen, break_stack);
            }
            code.push(Instruction::Label(end_label));
        }

        AST::Loop(block) => {
            let loop_start = label_gen.fresh("loop_start");
            let loop_end = label_gen.fresh("loop_end");
            break_stack.push(loop_end.clone());
            code.push(Instruction::PushImmutableContext);
            code.push(Instruction::Label(loop_start.clone()));
            code.push(Instruction::ClearImmutableContext);
            for stmt in block {
                compile(stmt, code, label_gen, break_stack);
            }
            code.push(Instruction::Jump(loop_start));
            code.push(Instruction::Label(loop_end));
            code.push(Instruction::PopImmutableContext);
            break_stack.pop();
        }

        AST::Break => {
            let target = break_stack
                .last()
                .expect("break used outside of loop")
                .clone();
            code.push(Instruction::Jump(target));
        }
    }
}

pub struct LabelGenerator {
    counter: usize,
}

impl LabelGenerator {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn fresh(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.counter);
        self.counter += 1;
        label
    }
}
