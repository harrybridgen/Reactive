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
    Not,
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
    Continue,
    Assert,
    Error,
}

//
// ----------------------------- AST -----------------------------
//
#[derive(Debug, Clone)]
pub enum CastType {
    Int,
    Char,
}
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
    Continue,
    Return(Option<Box<AST>>),
    // IO
    Print(Box<AST>),
    Println(Box<AST>),
    Assert(Box<AST>),
    Error(String),

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
    Cast {
        target: CastType,
        expr: Box<AST>,
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
pub enum CompiledStructFieldInit {
    Mutable(Vec<Instruction>),
    Immutable(Vec<Instruction>),
    Reactive(ReactiveExpr),
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
// ----------------------------- REACTIVE HELPERS -----------------------------
//

#[derive(Debug, Clone)]
pub struct ReactiveExpr {
    pub code: Vec<Instruction>,
    pub captures: Vec<String>,
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

    Function {
        params: Vec<String>,
        code: Vec<Instruction>,
    },

    LazyValue(ReactiveExpr, HashMap<String, Type>),
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
    StoreReactive(String, ReactiveExpr),

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
    StoreIndexReactive(String, ReactiveExpr),

    // structs
    StoreStruct(String, Vec<(String, Option<CompiledStructFieldInit>)>),
    NewStruct(String),
    FieldGet(String),
    FieldSet(String),
    FieldSetReactive(String, ReactiveExpr),
    FieldLValue(String),

    // indirect stores
    StoreThrough,
    StoreThroughReactive(ReactiveExpr),
    StoreThroughImmutable,

    // functions
    StoreFunction(String, Vec<String>, Vec<Instruction>),
    Call(String, usize),

    // immutable scopes
    PushImmutableContext,
    PopImmutableContext,
    ClearImmutableContext,

    // io
    Print,
    Println,
    Assert,
    Error(String),

    // modules
    Import(Vec<String>),

    //casts
    Cast(CastType),
}
