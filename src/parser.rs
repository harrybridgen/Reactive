use crate::grammar::{AST, Operator, StructFieldInit, Token, FieldAssignKind};

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, index: 0 }
    }

    fn next(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.index);
        self.index += 1;
        tok
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn peek_n(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.index + n)
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

    fn parse_factor(&mut self) -> AST {
        match self.next() {
            Some(Token::Ident(name)) => {
                let name = name.clone();

                if matches!(self.peek(), Some(Token::LParen)) {
                    self.next();

                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_or());
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

            Some(Token::Sub) => {
                let right = self.parse_factor();
                AST::Operation(
                    Box::new(AST::Number(0)),
                    Operator::Subtraction,
                    Box::new(right),
                )
            }

            Some(Token::LParen) => {
                let expr = self.parse_or();
                match self.next() {
                    Some(Token::RParen) => expr,
                    _ => panic!("[parse_factor] Expected ')'"),
                }
            }

            Some(Token::LSquare) => {
                let size_expr = self.parse_or();
                match self.next() {
                    Some(Token::RSquare) => AST::ArrayNew(Box::new(size_expr)),
                    _ => panic!("[parse_factor] Expected ']'"),
                }
            }

            Some(Token::Struct) => {
                let name = self.expect_ident();
                AST::StructNew(name)
            }

            other => panic!("[parse_factor] Could not parse factor: {:?}", other),
        }
    }

    fn parse_postfix(&mut self) -> AST {
        let mut ast = self.parse_factor();

        loop {
            match self.peek() {
                Some(Token::LSquare) => {
                    self.next();
                    let index_expr = self.parse_or();
                    match self.next() {
                        Some(Token::RSquare) => {
                            ast = AST::Index(Box::new(ast), Box::new(index_expr));
                        }
                        _ => panic!("[parse_postfix] Expected ']'"),
                    }
                }

                Some(Token::Dot) => {
                    self.next(); 
                    let field = self.expect_ident();
                    ast = AST::FieldAccess(Box::new(ast), field);
                }

                _ => break,
            }
        }

        ast
    }

    fn parse_summand(&mut self) -> AST {
        let mut ast = self.parse_postfix();

        while let Some(Token::Mul | Token::Div) = self.peek() {
            let op: Operator = match self.peek() {
                Some(Token::Mul) => Operator::Multiplication,
                Some(Token::Div) => Operator::Division,
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_postfix();
            ast = AST::Operation(Box::new(ast), op, Box::new(right));
        }

        ast
    }

    fn parse_expr(&mut self) -> AST {
        let mut ast = self.parse_summand();

        while let Some(Token::Add | Token::Sub) = self.peek() {
            let op: Operator = match self.peek() {
                Some(Token::Add) => Operator::Addition,
                Some(Token::Sub) => Operator::Subtraction,
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_summand();
            ast = AST::Operation(Box::new(ast), op, Box::new(right));
        }

        ast
    }

    fn parse_comparison(&mut self) -> AST {
        let mut ast = self.parse_expr();

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
            let right = self.parse_expr();
            ast = AST::Operation(Box::new(ast), op, Box::new(right));
        }

        ast
    }

    fn parse_and(&mut self) -> AST {
        let mut ast = self.parse_comparison();

        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::And => Operator::And,
                _ => break,
            };
            self.next();
            let right = self.parse_comparison();
            ast = AST::Operation(Box::new(ast), op, Box::new(right));
        }

        ast
    }

    fn parse_or(&mut self) -> AST {
        let mut ast = self.parse_and();

        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Or => Operator::Or,
                _ => break,
            };
            self.next();
            let right = self.parse_and();
            ast = AST::Operation(Box::new(ast), op, Box::new(right));
        }

        ast
    }

    fn parse_if(&mut self) -> AST {
        self.next(); 

        let cond = self.parse_or();
        let then_branch = self.parse_block();

        let else_branch = if let Some(Token::Else) = self.peek() {
            self.next();
            self.parse_block()
        } else {
            Vec::new()
        };

        AST::IfElse(Box::new(cond), then_branch, else_branch)
    }

    fn parse_block(&mut self) -> Vec<AST> {
        let mut statements = Vec::new();

        match self.next() {
            Some(Token::LBrace) => {}
            _ => panic!("Expected LBrace"),
        }

        while let Some(tok) = self.peek() {
            if matches!(tok, Token::RBrace) {
                break;
            }
            statements.push(self.parse_statement());
            if let Some(Token::Semicolon) = self.peek() {
                self.next();
            }
        }

        match self.next() {
            Some(Token::RBrace) => {}
            _ => panic!("Expected RBrace"),
        }

        statements
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
                    continue;
                }
                break;
            }
        }

        self.expect(Token::RParen);

        let body = self.parse_block();
        AST::FuncDef { name, params, body }
    }

    fn parse_struct_def(&mut self) -> AST {
        self.next();
        let name = self.expect_ident();

        match self.next() {
            Some(Token::LBrace) => {}
            other => panic!("Expected '{{' after struct name, got {:?}", other),
        }

        let mut fields: Vec<(String, Option<StructFieldInit>)> = Vec::new();

        while !matches!(self.peek(), Some(Token::RBrace)) {
            let field_name = self.expect_ident();

            let init = match self.peek() {
                Some(Token::Assign) => {
                    self.next();
                    Some(StructFieldInit::Mutable(self.parse_or()))
                }
                Some(Token::ImmutableAssign) => {
                    self.next();
                    Some(StructFieldInit::Immutable(self.parse_or()))
                }
                Some(Token::ReactiveAssign) => {
                    self.next();
                    Some(StructFieldInit::Reactive(self.parse_or()))
                }
                _ => None,
            };

            fields.push((field_name, init));

            if matches!(self.peek(), Some(Token::Semicolon)) {
                self.next();
            }
        }

        self.expect(Token::RBrace);

        AST::StructDef { name, fields }
    }

    fn parse_return(&mut self) -> AST {
        self.next(); 

        if matches!(self.peek(), Some(Token::Semicolon)) {
            return AST::Return(None);
        }

        match self.peek() {
            Some(Token::RBrace) | None => AST::Return(None),
            _ => {
                let expr = self.parse_or();
                AST::Return(Some(Box::new(expr)))
            }
        }
    }

    fn parse_statement(&mut self) -> AST {
        if let Some(Token::Func) = self.peek() {
            return self.parse_func_def();
        }
        if let Some(Token::Struct) = self.peek() {
            if matches!(self.peek_n(2), Some(Token::LBrace)) {
                return self.parse_struct_def();
            }
        }

        if let Some(Token::Return) = self.peek() {
            return self.parse_return();
        }

        if let Some(Token::Break) = self.peek() {
            self.next();
            return AST::Break;
        }

        if let Some(Token::If) = self.peek() {
            return self.parse_if();
        }

        if let Some(Token::Print) = self.peek() {
            self.next();
            let expr = self.parse_or();
            return AST::Print(Box::new(expr));
        }

        if let Some(Token::Println) = self.peek() {
            self.next();
            let expr = self.parse_or();
            return AST::Println(Box::new(expr));
        }

        if let Some(Token::Loop) = self.peek() {
            self.next();
            let loop_block = self.parse_block();
            return AST::Loop(loop_block);
        }

        if let Some(Token::Ident(name)) = self.peek() {
            if matches!(
                self.peek_n(1),
                Some(Token::Assign | Token::ReactiveAssign | Token::ImmutableAssign)
            ) {
                let name = name.clone();
                self.next();
                let op = self.next().cloned();
                let expr = self.parse_or();
                return match op {
                    Some(Token::Assign) => AST::Assign(name, Box::new(expr)),
                    Some(Token::ReactiveAssign) => AST::ReactiveAssign(name, Box::new(expr)),
                    Some(Token::ImmutableAssign) => AST::ImmutableAssign(name, Box::new(expr)),
                    _ => unreachable!(),
                };
            }

            if matches!(self.peek_n(1), Some(Token::LSquare)) {
                let arr_name = name.clone();
                self.next(); 

                self.expect(Token::LSquare);
                let index_expr = self.parse_or();
                self.expect(Token::RSquare);

                let op_tok = self.next().cloned();
                let value_expr = self.parse_or();

                return match op_tok {
                    Some(Token::Assign) => {
                        AST::AssignIndex(arr_name, Box::new(index_expr), Box::new(value_expr))
                    }
                    Some(Token::ReactiveAssign) => AST::ReactiveAssignIndex(
                        arr_name,
                        Box::new(index_expr),
                        Box::new(value_expr),
                    ),
                    Some(Token::ImmutableAssign) => {
                        panic!("[parse_statement] Immutable ':=' is not valid after arr[index]")
                    }
                    _ => panic!("[parse_statement] Expected '=' or '::=' after arr[index]"),
                };
            }
        }

        let expr = self.parse_or();

        match self.peek() {
            Some(Token::Assign) | Some(Token::ReactiveAssign) | Some(Token::ImmutableAssign) => {
                let op = self.next().cloned().unwrap();
                let rhs = self.parse_or();

                if let AST::FieldAccess(base, field) = expr {

                    match op {
                        Token::Assign | Token::ReactiveAssign | Token::ImmutableAssign => {
                            let kind = match op {
                                Token::Assign => FieldAssignKind::Normal,
                                Token::ReactiveAssign => FieldAssignKind::Reactive,
                                Token::ImmutableAssign => FieldAssignKind::Immutable,
                                _ => unreachable!(),
                            };

                            return AST::FieldAssign {
                                base,
                                field,
                                value: Box::new(rhs),
                                kind,
                            };
                        }
                        _ => unreachable!(),
                    }
                } else {
                    panic!("Left-hand side of field assignment must be a field access");
                }
            }
            _ => {}
        }

        expr
    }

    fn parse_program(&mut self) -> AST {
        let mut statements = Vec::new();

        while self.peek().is_some() {
            statements.push(self.parse_statement());
            if let Some(Token::Semicolon) = self.peek() {
                self.next();
            }
        }

        AST::Program(statements)
    }
}

pub fn parse(tokens: Vec<Token>) -> AST {
    let mut parser: Parser = Parser::new(tokens);
    let result = parser.parse_program();
    if parser.index != parser.tokens.len() {
        panic!("Failed to consume all tokens!")
    }
    result
}
