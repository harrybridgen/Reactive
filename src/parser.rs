use crate::grammar::{AST, Operator, StructFieldInit, Token};

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn peek_n(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.index + n)
    }

    fn next(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.index);
        self.index += 1;
        tok
    }

    fn expect(&mut self, expected: Token) {
        let got = self.next().cloned();
        if got.as_ref() != Some(&expected) {
            panic!("Expected {:?}, got {:?}", expected, got);
        }
    }

    fn expect_ident(&mut self) -> String {
        match self.next() {
            Some(Token::Ident(s)) => s.clone(),
            other => panic!("Expected identifier, got {:?}", other),
        }
    }

    // ---------------- expressions ----------------

    fn parse_factor(&mut self) -> AST {
        match self.next() {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.next();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_ternary());
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen);
                    AST::Call { name, args }
                } else {
                    AST::Var(name)
                }
            }

            Some(Token::Number(n)) => AST::Number(*n),
            Some(Token::Char(c)) => AST::Char(*c),
            Some(Token::StringLiteral(s)) => AST::StringLiteral(s.clone()),

            Some(Token::LParen) => {
                let expr = self.parse_ternary();
                self.expect(Token::RParen);
                expr
            }

            Some(Token::LSquare) => {
                let size = self.parse_ternary();
                self.expect(Token::RSquare);
                AST::ArrayNew(Box::new(size))
            }

            Some(Token::Struct) => {
                let name = self.expect_ident();
                AST::StructNew(name)
            }

            other => panic!("[parse_factor] invalid token {:?}", other),
        }
    }

    fn parse_postfix(&mut self) -> AST {
        let mut expr = self.parse_factor();
        loop {
            match self.peek() {
                Some(Token::LSquare) => {
                    self.next();
                    let idx = self.parse_ternary();
                    self.expect(Token::RSquare);
                    expr = AST::Index(Box::new(expr), Box::new(idx));
                }
                Some(Token::Dot) => {
                    self.next();
                    let field = self.expect_ident();
                    expr = AST::FieldAccess(Box::new(expr), field);
                }
                _ => break,
            }
        }
        expr
    }

    fn parse_unary(&mut self) -> AST {
        if matches!(self.peek(), Some(Token::Sub)) {
            self.next();
            AST::Operation(
                Box::new(AST::Number(0)),
                Operator::Subtraction,
                Box::new(self.parse_unary()),
            )
        } else {
            self.parse_postfix()
        }
    }

    fn parse_mul(&mut self) -> AST {
        let mut expr = self.parse_unary();
        while let Some(Token::Mul | Token::Div | Token::Modulo) = self.peek() {
            let op = match self.next() {
                Some(Token::Mul) => Operator::Multiplication,
                Some(Token::Div) => Operator::Division,
                Some(Token::Modulo) => Operator::Modulo,
                _ => unreachable!(),
            };
            let rhs = self.parse_unary();
            expr = AST::Operation(Box::new(expr), op, Box::new(rhs));
        }
        expr
    }

    fn parse_add(&mut self) -> AST {
        let mut expr = self.parse_mul();
        while let Some(Token::Add | Token::Sub) = self.peek() {
            let op = match self.next() {
                Some(Token::Add) => Operator::Addition,
                Some(Token::Sub) => Operator::Subtraction,
                _ => unreachable!(),
            };
            let rhs = self.parse_mul();
            expr = AST::Operation(Box::new(expr), op, Box::new(rhs));
        }
        expr
    }

    fn parse_cmp(&mut self) -> AST {
        let mut expr = self.parse_add();
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Greater => Operator::Greater,
                Token::Less => Operator::Less,
                Token::Equal => Operator::Equal,
                Token::GreaterEqual => Operator::GreaterEqual,
                Token::LessEqual => Operator::LessEqual,
                Token::NotEqual => Operator::NotEqual,
                _ => break,
            };
            self.next();
            let rhs = self.parse_add();
            expr = AST::Operation(Box::new(expr), op, Box::new(rhs));
        }
        expr
    }

    fn parse_and(&mut self) -> AST {
        let mut expr = self.parse_cmp();
        while matches!(self.peek(), Some(Token::And)) {
            self.next();
            let rhs = self.parse_cmp();
            expr = AST::Operation(Box::new(expr), Operator::And, Box::new(rhs));
        }
        expr
    }

    fn parse_or(&mut self) -> AST {
        let mut expr = self.parse_and();
        while matches!(self.peek(), Some(Token::Or)) {
            self.next();
            let rhs = self.parse_and();
            expr = AST::Operation(Box::new(expr), Operator::Or, Box::new(rhs));
        }
        expr
    }

    fn parse_ternary(&mut self) -> AST {
        let cond = self.parse_or();
        if matches!(self.peek(), Some(Token::Question)) {
            self.next();
            let then_expr = self.parse_ternary();
            self.expect(Token::Colon);
            let else_expr = self.parse_ternary();
            AST::Ternary {
                cond: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            }
        } else {
            cond
        }
    }

    // ---------------- statements ----------------

    fn parse_block(&mut self) -> Vec<AST> {
        self.expect(Token::LBrace);
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace)) {
            stmts.push(self.parse_statement());
            if matches!(self.peek(), Some(Token::Semicolon)) {
                self.next();
            }
        }
        self.expect(Token::RBrace);
        stmts
    }

    fn parse_if(&mut self) -> AST {
        self.next();
        let cond = self.parse_ternary();
        let then_block = self.parse_block();
        let else_block = if matches!(self.peek(), Some(Token::Else)) {
            self.next();
            self.parse_block()
        } else {
            Vec::new()
        };
        AST::IfElse(Box::new(cond), then_block, else_block)
    }

    fn parse_func_def(&mut self) -> AST {
        self.next();
        let name = self.expect_ident();
        self.expect(Token::LParen);
        let mut params = Vec::new();
        if !matches!(self.peek(), Some(Token::RParen)) {
            loop {
                params.push(self.expect_ident());
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.next();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen);
        let body = self.parse_block();
        AST::FuncDef { name, params, body }
    }

    fn parse_struct_def(&mut self) -> AST {
        self.next();
        let name = self.expect_ident();
        self.expect(Token::LBrace);

        let mut fields = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace)) {
            let fname = self.expect_ident();
            let init = match self.peek() {
                Some(Token::Assign) => {
                    self.next();
                    Some(StructFieldInit::Mutable(self.parse_ternary()))
                }
                Some(Token::ImmutableAssign) => {
                    self.next();
                    Some(StructFieldInit::Immutable(self.parse_ternary()))
                }
                Some(Token::ReactiveAssign) => {
                    self.next();
                    Some(StructFieldInit::Reactive(self.parse_ternary()))
                }
                _ => None,
            };
            fields.push((fname, init));
            if matches!(self.peek(), Some(Token::Semicolon)) {
                self.next();
            }
        }

        self.expect(Token::RBrace);
        AST::StructDef { name, fields }
    }

    fn parse_return(&mut self) -> AST {
        self.next();
        if matches!(self.peek(), Some(Token::Semicolon | Token::RBrace)) || self.peek().is_none() {
            AST::Return(None)
        } else {
            AST::Return(Some(Box::new(self.parse_ternary())))
        }
    }

    fn parse_statement(&mut self) -> AST {
        match self.peek() {
            Some(Token::Import) => {
                self.next();
                let mut path = vec![self.expect_ident()];
                while matches!(self.peek(), Some(Token::Dot)) {
                    self.next();
                    path.push(self.expect_ident());
                }
                AST::Import(path)
            }

            Some(Token::Func) => self.parse_func_def(),

            Some(Token::Struct) if matches!(self.peek_n(2), Some(Token::LBrace)) => {
                self.parse_struct_def()
            }

            Some(Token::Return) => self.parse_return(),

            Some(Token::Break) => {
                self.next();
                AST::Break
            }

            Some(Token::If) => self.parse_if(),

            Some(Token::Print) => {
                self.next();
                AST::Print(Box::new(self.parse_ternary()))
            }

            Some(Token::Println) => {
                self.next();
                AST::Println(Box::new(self.parse_ternary()))
            }

            Some(Token::Loop) => {
                self.next();
                AST::Loop(self.parse_block())
            }

            Some(Token::Ident(name))
                if matches!(
                    self.peek_n(1),
                    Some(Token::Assign | Token::ReactiveAssign | Token::ImmutableAssign)
                ) =>
            {
                let name = name.clone();
                self.next();
                let op = self.next().cloned().unwrap();
                let rhs = self.parse_ternary();
                match op {
                    Token::Assign => AST::Assign(name, Box::new(rhs)),
                    Token::ReactiveAssign => AST::ReactiveAssign(name, Box::new(rhs)),
                    Token::ImmutableAssign => AST::ImmutableAssign(name, Box::new(rhs)),
                    _ => unreachable!(),
                }
            }

            _ => {
                let lhs = self.parse_ternary();
                match self.peek() {
                    Some(Token::Assign) => {
                        self.next();
                        AST::AssignTarget(Box::new(lhs), Box::new(self.parse_ternary()))
                    }
                    Some(Token::ReactiveAssign) => {
                        self.next();
                        AST::ReactiveAssignTarget(Box::new(lhs), Box::new(self.parse_ternary()))
                    }
                    Some(Token::ImmutableAssign) => {
                        self.next();
                        AST::ImmutableAssignTarget(Box::new(lhs), Box::new(self.parse_ternary()))
                    }
                    _ => lhs,
                }
            }
        }
    }

    fn parse_toplevel(&mut self) -> AST {
        match self.peek() {
            Some(Token::Import) => {
                self.next();
                let mut path = vec![self.expect_ident()];
                while matches!(self.peek(), Some(Token::Dot)) {
                    self.next();
                    path.push(self.expect_ident());
                }
                AST::Import(path)
            }

            Some(Token::Func) => self.parse_func_def(),

            Some(Token::Struct) if matches!(self.peek_n(2), Some(Token::LBrace)) => {
                self.parse_struct_def()
            }

            Some(Token::Ident(_)) if matches!(self.peek_n(1), Some(Token::ImmutableAssign)) => {
                let name = self.expect_ident();
                self.expect(Token::ImmutableAssign);
                let rhs = self.parse_ternary();
                AST::ImmutableAssign(name, Box::new(rhs))
            }

            other => panic!("invalid top-level item: {:?}", other),
        }
    }

    fn parse_program(&mut self) -> AST {
        let mut items = Vec::new();
        while self.peek().is_some() {
            items.push(self.parse_toplevel());
            if matches!(self.peek(), Some(Token::Semicolon)) {
                self.next();
            }
        }
        AST::Program(items)
    }
}

pub fn parse(tokens: Vec<Token>) -> AST {
    let mut parser = Parser::new(tokens);
    let ast = parser.parse_program();
    if parser.index != parser.tokens.len() {
        panic!("parser did not consume all tokens");
    }
    ast
}
