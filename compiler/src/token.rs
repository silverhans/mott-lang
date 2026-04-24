#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Float(f64),
    String(Vec<StringPart>),
    Ident(String),

    // Keywords
    Fnc,
    Xilit,
    Yuxadalo,
    Yazde,
    NagahSanna,
    Khi,
    Cqachunna,
    Sac,
    Khida,
    Baqderg,
    Xarco,
    A,
    Ya,

    // Type keywords
    Terah,
    Bool,
    Deshnash,
    Daqosh,

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Comma,
    Colon,
    Arrow,

    // Operators
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Not,

    Eof,
}

/// A fragment of a string literal: either raw text or a `{ident}` interpolation.
/// Produced at lex time because mott's interpolation grammar is purely lexical
/// (only bare identifiers are allowed inside `{...}`).
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(String),
    Interpolation(String),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}
