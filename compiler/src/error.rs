use std::fmt;

#[derive(Debug)]
pub enum Error {
    Lex { line: usize, col: usize, message: String },
    Parse { line: usize, col: usize, message: String },
    /// Semantic analysis: type errors, scope violations, undefined symbols,
    /// language-rule violations (sac outside loop, push on parameter, etc.).
    /// Source positions aren't tracked yet — sema walks the AST after lex/
    /// parse so the positions on AST nodes would need plumbing through.
    /// Adding positions is a quick win when it becomes a pain.
    Sema(String),
    /// Genuine codegen failure (very rare — most "codegen errors" are
    /// actually sema errors that used to live in the C backend).
    Codegen(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lex { line, col, message } => {
                write!(f, "lex error at {}:{}: {}", line, col, message)
            }
            Error::Parse { line, col, message } => {
                write!(f, "parse error at {}:{}: {}", line, col, message)
            }
            Error::Sema(msg) => write!(f, "sema error: {}", msg),
            Error::Codegen(msg) => write!(f, "codegen error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
