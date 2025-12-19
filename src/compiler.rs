use crate::grammar::{AST, Instruction, Operator};

pub fn compile(
    ast: AST,
    code: &mut Vec<Instruction>,
    label_gen: &mut LabelGenerator,
    break_stack: &mut Vec<String>,
) {
    match ast {
        AST::Number(n) => code.push(Instruction::Push(n)),

        AST::Var(name) => code.push(Instruction::Load(name)),

        AST::Operation(left, oper, right) => {
            compile(*left, code, label_gen, break_stack);
            compile(*right, code, label_gen, break_stack);

            match oper {
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

        AST::Assign(name, ast) => {
            compile(*ast, code, label_gen, break_stack);
            code.push(Instruction::Store(name));
        }

        AST::LazyAssign(name, ast) => {
            code.push(Instruction::StoreLazy(name, ast));
        }

        AST::Print(ast) => {
            compile(*ast, code, label_gen, break_stack);
            code.push(Instruction::Print);
        }

        AST::Program(statements) => {
            for stmt in statements {
                compile(stmt, code, label_gen, break_stack);
            }
        }

        AST::IfElse(cond, ifbranch, elsebranch) => {
            compile(*cond, code, label_gen, break_stack);

            let else_label = label_gen.fresh("else");
            let end_label = label_gen.fresh("end");

            code.push(Instruction::JumpIfZero(else_label.clone()));

            for stmt in ifbranch {
                compile(stmt, code, label_gen, break_stack);
            }

            code.push(Instruction::Jump(end_label.clone()));
            code.push(Instruction::Label(else_label));

            for stmt in elsebranch {
                compile(stmt, code, label_gen, break_stack);
            }

            code.push(Instruction::Label(end_label));
        }

        AST::Loop(block) => {
            let loop_start = label_gen.fresh("loop_start");
            let loop_end = label_gen.fresh("loop_end");

            break_stack.push(loop_end.clone());

            code.push(Instruction::Label(loop_start.clone()));

            for stmt in block {
                compile(stmt, code, label_gen, break_stack);
            }

            code.push(Instruction::Jump(loop_start));
            code.push(Instruction::Label(loop_end));

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
