use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum CastType {
    Int,
    Char,
}

#[derive(Debug, Clone)]
pub struct ReactiveExpr {
    pub code: Vec<Instruction>,
    pub captures: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Type {
    Integer(i32),
    Char(u32),

    ArrayRef(usize),
    VecRef(usize),
    BufferRef(usize),
    StructRef(usize),

    Function {
        params: Vec<String>,
        code: Vec<Instruction>,
    },
    NativeFunction(String),

    LazyValue(ReactiveExpr, HashMap<String, Type>),
    LValue(LValue),
    Uninitialized,
}

#[derive(Debug, Clone)]
pub enum LValue {
    ArrayElem { array_id: usize, index: usize },
    VecElem { vec_id: usize, index: usize },
    StructField { struct_id: usize, field: String },
}

#[derive(Debug, Clone)]
pub struct StructInstance {
    pub fields: HashMap<String, Type>,
    pub immutables: HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum CompiledStructFieldInit {
    Mutable(Vec<Instruction>),
    Immutable(Vec<Instruction>),
    Reactive(ReactiveExpr),
}

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

    // casts
    Cast(CastType),
}
