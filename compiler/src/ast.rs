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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Terah,    // int64
    Bool,     // bool
    Deshnash, // string
    Daqosh,   // float64
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
    If {
        cond: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    Return(Option<Expr>),
    Print(Expr),
    ExprStmt(Expr),
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
