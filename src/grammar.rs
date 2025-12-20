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

#[derive(Debug, Clone)]
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
    Println,
    Greater,
    Less,
    Equal,
    Or,
    And,
    If,
    Else,
    LBrace,
    RBrace, LSquare, RSquare,
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
    LazyInteger(Box<AST>),Array(Vec<Type>),
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
    Println(Box<AST>),
    IfElse(Box<AST>, Vec<AST>, Vec<AST>),
    Loop(Vec<AST>),
    Break,

    ArrayNew(Box<AST>),                  
    Index(Box<AST>, Box<AST>),          
    AssignIndex(String, Box<AST>, Box<AST>),    
    LazyAssignIndex(String, Box<AST>, Box<AST>),
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

#[derive(Debug, Clone)]
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
    Println,
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
    
    ArrayNew, // pops Integer(size), pushes Array(len=size, init 0)
    ArrayGet, // pops index(Integer) and array(Array), pushes element (evaluated to Integer if lazy)
    StoreIndex(String),              // pops value, pops index(Integer); mutates env[name] as array
    StoreIndexLazy(String, Box<AST>),// pops index(Integer); stores LazyInteger(ast) into env[name][index]
}
