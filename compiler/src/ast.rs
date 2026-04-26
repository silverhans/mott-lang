/// Part of an interpolated string literal, as stored in the AST.
///
/// Distinct from `token::StringPart` because the lexer captures the raw
/// source text inside `{...}` and the parser re-parses it into an `Expr`.
/// Upstream: `StringPart::Literal("x = ")`, `StringPart::Interpolation(x)`.
#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Interpolation(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Struct(StructDef),
    /// `eca name` — bring module into scope. Resolved by the loader
    /// before sema sees the program; after loading, imports are still
    /// here for diagnostics but their content has been merged in as
    /// regular Items with `module = Some(name)` set.
    Import { module: String },
}

/// A `kep` declaration. Field order is preserved (matters for codegen
/// layout, not for the language — fields are accessed by name).
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    /// Same role as `Function::module` — owning module of this struct.
    /// User-defined: None. Imported from stdlib or another file: Some.
    pub module: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    /// `None` means "extern" — body lives in the C runtime, signature only.
    /// User code always has `Some(block)`; module-loaded stdlib functions
    /// (e.g. `math.sqrt`) are typically `None`.
    pub body: Option<Block>,
    /// `None` for user-level functions; `Some("math")` for functions
    /// loaded from a module. Used for name-mangling at codegen and for
    /// validating qualified call sites in sema.
    pub module: Option<String>,
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
    /// User-defined struct, referenced by name. Resolved against
    /// `kep` declarations during sema.
    Struct(String),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `xilit x = 5` — init provided, type inferred or annotated.
    /// `xilit x: T` — type-annotated, no init. Codegen zero-initializes
    /// based on T. A Let with both `ty == None` and `value == None` is
    /// a parse error (no type to infer from).
    Let {
        name: String,
        ty: Option<Type>,
        value: Option<Expr>,
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
    /// `push(arr, value)` — append to a dynamic array. Parser constrains
    /// the first arg to a bare identifier because we need it as an
    /// l-value (the runtime takes `&arr` to update data/len/cap atomically).
    Push { name: String, value: Expr },
    /// `var.field = expr` — assign to a struct field. Like IndexAssign,
    /// the target is restricted to a bare identifier in v0.3 (no chains
    /// like `line.start.x = ...`); the workaround is local + reassign.
    FieldAssign {
        target: String,
        field: String,
        value: Expr,
    },
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
        /// `None` for plain calls (`foo(1, 2)`); `Some("math")` for
        /// qualified calls (`math.sqrt(2.0)`). Resolution happens in
        /// sema against the imported module table.
        module: Option<String>,
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
    /// `parse_terah(s)` — parse a deshnash into a terah. Aborts at runtime
    /// on malformed input (empty, trailing garbage, overflow). No Result
    /// type yet; that's a post-MVP addition.
    ParseTerah(Box<Expr>),
    /// `parse_daqosh(s)` — same as ParseTerah, but for daqosh. Uses strtod
    /// so accepts the usual float forms (decimal, exponent, infinity, nan).
    ParseDaqosh(Box<Expr>),
    /// `to_terah(x)` — numeric -> terah. Lowered to a C cast; truncates
    /// toward zero for floats. Out-of-range floats (`inf`, `nan`, or
    /// values above `int64` max) are UB per C — a runtime-checked variant
    /// will come with Result.
    ToTerah(Box<Expr>),
    /// `to_daqosh(x)` — numeric -> daqosh. Lowered to a C cast. Converting
    /// large int64 values loses precision (standard IEEE 754 behavior).
    ToDaqosh(Box<Expr>),
    /// `pop(arr)` — remove and return the last element of a dynamic array.
    /// Same l-value restriction as push: first arg must be a bare ident so
    /// the runtime can update len in place. Runtime-aborts on empty array.
    Pop(String),
    /// `Name { field: value, ... }` — struct construction. All fields
    /// must be supplied (no defaults yet). Order in the AST follows
    /// source order; sema reorders for type-check / codegen as needed.
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    /// `target.field` — read a struct field.
    FieldAccess {
        target: Box<Expr>,
        field: String,
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
