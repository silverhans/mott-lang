use crate::token::StringPart;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Terah,            // int64
    Bool,             // bool
    Deshnash,         // string
    Daqosh,           // float64
    Array(Box<Type>), // [T] — heap-backed array, fixed-size in MVP
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    /// `arr[idx] = value` — write into an array element. Kept as a separate
    /// statement (not a general l-value) to avoid dragging a full l-value
    /// system into the parser for a single feature.
    IndexAssign {
        name: String,
        index: Expr,
        value: Expr,
    },
    If {
        cond: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    /// `yallalc var chu source { body }`. `source` is either an array
    /// expression or a range; the variant distinguishes which.
    ForEach {
        var: String,
        iter: IterSource,
        body: Block,
    },
    Break,
    Continue,
    Return(Option<Expr>),
    Print(Expr),
    ExprStmt(Expr),
}

#[derive(Debug, Clone)]
pub enum IterSource {
    /// `chu arr_expr` — iterate elements of an array.
    Array(Expr),
    /// `chu start..end` — half-open integer range, end exclusive.
    Range { start: Expr, end: Expr },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(Vec<StringPart>),
    Ident(String),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    LogicAnd(Vec<Expr>),
    LogicOr(Vec<Expr>),
    Call {
        callee: String,
        args: Vec<Expr>,
    },
    /// `esha()` — reads one line from stdin, returns `deshnash`. Trailing
    /// newline stripped. On EOF, returns an empty string.
    Input,
    /// `[e1, e2, ...]` — array literal. Empty literals (`[]`) are ambiguous
    /// about element type; we require at least one element for now.
    ArrayLit(Vec<Expr>),
    /// `target[index]` — array element access. Returns the element type.
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    /// `baram(x)` — size/length built-in. Works on arrays and strings; the
    /// codegen picks the right struct field at emission time.
    Baram(Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
}
