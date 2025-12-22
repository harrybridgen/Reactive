#[derive(Debug, Clone, PartialEq)]
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
    ReactiveAssign,
    ImmutableAssign,
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
    RBrace,
    LSquare,
    RSquare,
    Loop,
    Break,
    LessEqual,
    GreaterEqual,
    NotEqual,
    Not,
    Func,
    Return,
    Struct,
    Dot,
    Comma,
    Import,
    Modulo,
    Question,
    Colon,
    Char(u32),
    StringLiteral(String),
}

use std::collections::HashMap;
use std::collections::HashSet;
#[derive(Debug, Clone)]
pub enum Type {
    Integer(i32),
    LazyValue(Box<AST>),
    ArrayRef(usize),

    Function { params: Vec<String>, body: Vec<AST> },

    StructRef(usize),
    LValue(LValue),
    Char(u32),
}
#[derive(Debug, Clone)]
pub enum LValue {
    ArrayElem { array_id: usize, index: usize },
    StructField { struct_id: usize, field: String },
}
#[derive(Debug, Clone)]
pub struct StructInstance {
    pub fields: HashMap<String, Type>,
    pub immutables: HashSet<String>,
}
#[derive(Debug, Clone)]
pub enum AST {
    Number(i32),
    Operation(Box<AST>, Operator, Box<AST>),
    Assign(String, Box<AST>),
    ReactiveAssign(String, Box<AST>),
    ImmutableAssign(String, Box<AST>),
    Var(String),
    Program(Vec<AST>),
    Print(Box<AST>),
    Println(Box<AST>),
    IfElse(Box<AST>, Vec<AST>, Vec<AST>),
    Loop(Vec<AST>),
    Break,
    Import(Vec<String>),

    ArrayNew(Box<AST>),
    Index(Box<AST>, Box<AST>),
    AssignTarget(Box<AST>, Box<AST>),
    ReactiveAssignTarget(Box<AST>, Box<AST>),
    FuncDef {
        name: String,
        params: Vec<String>,
        body: Vec<AST>,
    },

    Return(Option<Box<AST>>),

    Call {
        name: String,
        args: Vec<AST>,
    },

    StructDef {
        name: String,
        fields: Vec<(String, Option<StructFieldInit>)>,
    },

    StructNew(String),

    FieldAccess(Box<AST>, String),
    FieldAssign {
        base: Box<AST>,
        field: String,
        value: Box<AST>,
        kind: FieldAssignKind,
    },
    Ternary {
        cond: Box<AST>,
        then_expr: Box<AST>,
        else_expr: Box<AST>,
    },
    Char(u32),
    StringLiteral(String),
}
#[derive(Debug, Clone)]
pub enum FieldAssignKind {
    Normal,
    Reactive,
    Immutable,
}

#[derive(Debug, Clone)]
pub enum StructFieldInit {
    Mutable(AST),
    Immutable(AST),
    Reactive(AST),
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
    Modulo,
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
    StoreReactive(String, Box<AST>),
    StoreImmutable(String),
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

    Modulo,

    ArrayNew,
    ArrayGet,
    StoreIndex(String),
    StoreIndexReactive(String, Box<AST>),
    PushImmutableContext,
    PopImmutableContext,
    ClearImmutableContext,

    StoreFunction(String, Vec<String>, Vec<AST>),

    StoreStruct(String, Vec<(String, Option<StructFieldInit>)>),
    NewStruct(String),
    FieldGet(String),
    FieldSet(String),
    FieldSetReactive(String, Box<AST>),

    Call(String, usize),

    ArrayLValue,
    FieldLValue(String),
    StoreThrough,
    StoreThroughReactive(Box<AST>),
    Import(Vec<String>),
    Return,
    PushChar(u32),
}
