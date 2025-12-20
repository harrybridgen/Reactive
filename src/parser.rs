use crate::grammar::{AST, Operator, Token};

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, index: 0 }
    }

    fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.index);
        self.index += 1;
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

fn parse_factor(&mut self) -> AST {
    match self.next() {
        Some(Token::Ident(name)) => AST::Var(name.clone()),

        Some(Token::Number(n)) => AST::Number(*n),

        Some(Token::Sub) => {
            let right = self.parse_factor();
            AST::Operation(Box::new(AST::Number(0)), Operator::Subtraction, Box::new(right))
        }

        Some(Token::LParen) => {
            let expr = self.parse_or();
            match self.next() {
                Some(Token::RParen) => expr,
                _ => panic!("[parse_factor] Could not find right bracket"),
            }
        }

        Some(Token::LSquare) => {
            let size_expr = self.parse_or();
            match self.next() {
                Some(Token::RSquare) => AST::ArrayNew(Box::new(size_expr)),
                _ => panic!("[parse_factor] Expected ']'"),
            }
        }

        _ => panic!("[parse_factor] Could not parse factor"),
    }
}

fn parse_postfix(&mut self) -> AST {
    let mut ast = self.parse_factor();

    while let Some(Token::LSquare) = self.peek() {
        self.next();
        let index_expr = self.parse_or();
        match self.next() {
            Some(Token::RSquare) => {
                ast = AST::Index(Box::new(ast), Box::new(index_expr));
            }
            _ => panic!("[parse_postfix] Expected ']'"),
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
            _ => panic!("[parse_summand] Could not parse Operation"),
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
                _ => panic!("[parse_summand] Could not parse Operation"),
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

    fn parse_statement(&mut self) -> AST {
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
    if matches!(self.tokens.get(self.index + 1), Some(Token::Assign | Token::LazyAssign)) {
        let name = name.clone();
        self.next(); 
        let is_lazy = matches!(self.next(), Some(Token::LazyAssign));
        let expr = self.parse_or();
        return if is_lazy {
            AST::LazyAssign(name, Box::new(expr))
        } else {
            AST::Assign(name, Box::new(expr))
        };
    }

if matches!(self.tokens.get(self.index + 1), Some(Token::LSquare)) {
    let arr_name = name.clone();
    self.next();

    match self.next() {
        Some(Token::LSquare) => {}
        _ => panic!("[parse_statement] Expected '[' after array name"),
    }

    let index_expr = self.parse_or();

    match self.next() {
        Some(Token::RSquare) => {}
        _ => panic!("[parse_statement] Expected ']' after index"),
    }

    let op_tok = self.next().cloned();
    let value_expr = self.parse_or();

    return match op_tok {
        Some(Token::Assign) => {
            AST::AssignIndex(arr_name, Box::new(index_expr), Box::new(value_expr))
        }
        Some(Token::LazyAssign) => {
            AST::LazyAssignIndex(arr_name, Box::new(index_expr), Box::new(value_expr))
        }
        _ => panic!("[parse_statement] Expected '=' or ':=' after arr[index]"),
    };
}

}


        self.parse_or()
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
