use std::collections::{HashMap, HashSet};

//
// ----------------------------- TOKENS -----------------------------
//

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // literals / identifiers
    Number(i32),
    Ident(String),
    Char(u32),
    StringLiteral(String),

    // arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Modulo,

    // comparison / logic
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Equal,
    NotEqual,
    And,
    Or,

    // assignment
    Assign,
    ImmutableAssign,
    ReactiveAssign,

    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LSquare,
    RSquare,
    Semicolon,
    Dot,
    Comma,
    Colon,
    Question,

    // keywords
    If,
    Else,
    Loop,
    Break,
    Func,
    Return,
    Struct,
    Import,
    Print,
    Println,
}

//
// ----------------------------- RUNTIME TYPES -----------------------------
//

#[derive(Debug, Clone)]
pub enum Type {
    Integer(i32),
    Char(u32),

    ArrayRef(usize),
    StructRef(usize),

    Function { params: Vec<String>, body: Vec<AST> },

    LazyValue(Box<AST>, HashMap<String, Type>),
    LValue(LValue),
    Uninitialized,
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

//
// ----------------------------- AST -----------------------------
//

#[derive(Debug, Clone)]
pub enum AST {
    // literals
    Number(i32),
    Char(u32),
    StringLiteral(String),

    // variables
    Var(String),

    // expressions
    Operation(Box<AST>, Operator, Box<AST>),
    Ternary {
        cond: Box<AST>,
        then_expr: Box<AST>,
        else_expr: Box<AST>,
    },

    // arrays
    ArrayNew(Box<AST>),
    Index(Box<AST>, Box<AST>),

    // assignment (binding-level)
    Assign(String, Box<AST>),
    ImmutableAssign(String, Box<AST>),
    ReactiveAssign(String, Box<AST>),
    ImmutableAssignTarget(Box<AST>, Box<AST>),

    // assignment (lvalue-level)
    AssignTarget(Box<AST>, Box<AST>),
    ReactiveAssignTarget(Box<AST>, Box<AST>),

    // control flow
    Program(Vec<AST>),
    IfElse(Box<AST>, Vec<AST>, Vec<AST>),
    Loop(Vec<AST>),
    Break,
    Return(Option<Box<AST>>),

    // IO
    Print(Box<AST>),
    Println(Box<AST>),

    // functions
    FuncDef {
        name: String,
        params: Vec<String>,
        body: Vec<AST>,
    },
    Call {
        name: String,
        args: Vec<AST>,
    },

    // structs
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

    // modules
    Import(Vec<String>),
}

//
// ----------------------------- STRUCT FIELDS -----------------------------
//

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

//
// ----------------------------- OPERATORS -----------------------------
//

#[derive(Debug, Clone)]
pub enum Operator {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Modulo,

    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Equal,
    NotEqual,

    And,
    Or,
}

//
// ----------------------------- BYTECODE -----------------------------
//

#[derive(Debug, Clone)]
pub enum Instruction {
    // stack ops
    Push(i32),
    PushChar(u32),
    Load(String),

    // variable storage
    Store(String),
    StoreImmutable(String),
    StoreReactive(String, Box<AST>),

    // arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Modulo,

    // comparison / logic
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Equal,
    NotEqual,
    And,
    Or,

    // control flow
    Label(String),
    Jump(String),
    JumpIfZero(String),
    Return,

    // arrays
    ArrayNew,
    ArrayGet,
    ArrayLValue,
    StoreIndex(String),
    StoreIndexReactive(String, Box<AST>),

    // structs
    StoreStruct(String, Vec<(String, Option<StructFieldInit>)>),
    NewStruct(String),
    FieldGet(String),
    FieldSet(String),
    FieldSetReactive(String, Box<AST>),
    FieldLValue(String),

    // indirect stores
    StoreThrough,
    StoreThroughReactive(Box<AST>),
    StoreThroughImmutable,

    // functions
    StoreFunction(String, Vec<String>, Vec<AST>),
    Call(String, usize),

    // immutable scopes
    PushImmutableContext,
    PopImmutableContext,
    ClearImmutableContext,

    // io
    Print,
    Println,

    // modules
    Import(Vec<String>),
}
