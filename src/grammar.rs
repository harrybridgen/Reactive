// program     ::= statement (';' statement)* ';'?

// statement   ::= identifier "=" expression
//               | identifier ":=" expression
//               | print expr
//               | expression

// expression  ::= summand (("+" | "-") summand)*

// summand     ::= factor (("*" | "/") factor)*

// factor      ::= "-" factor
//               | number
//               | identifier
//               | "(" expression ")"

#[derive(Debug)]
pub enum Token {
    Add,
    Mul,
    Div,
    Sub,
    Number(i32),
    LParen,
    RParen,
    Ident(String),
    Assign,
    LazyAssign,
    Semicolon,
    Print,
    Greater,
    Less,
    Equal,
    Or,
    And,
    If,
    Else,
    LBrace,
    RBrace,
    Loop,
    Break,
    LessEqual,
    GreaterEqual,
    NotEqual,
    Not,
}

#[derive(Debug, Clone)]
pub enum Type {
    Integer(i32),
    LazyInteger(Box<AST>),
}

#[derive(Debug, Clone)]
pub enum AST {
    Number(i32),
    Operation(Box<AST>, Operator, Box<AST>),
    Assign(String, Box<AST>),
    LazyAssign(String, Box<AST>),
    Var(String),
    Program(Vec<AST>),
    Print(Box<AST>),
    IfElse(Box<AST>, Vec<AST>, Vec<AST>),
    Loop(Vec<AST>),Break
}

#[derive(Debug, Clone)]
pub enum Operator {
    Addition,
    Multiplication,
    Division,
    Subtraction,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    NotEqual,
    Equal,
    Or,
    And,
}

#[derive(Debug)]
pub enum Instruction {
    Add,
    Mul,
    Div,
    Sub,
    Push(i32),
    Load(String),
    Store(String),
    StoreLazy(String, Box<AST>),
    Print,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Equal,
    NotEqual,
    Or,
    And,
    Label(String),
    Jump(String),
    JumpIfZero(String),
}
