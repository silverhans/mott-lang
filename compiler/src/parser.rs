use crate::ast::Program;
use crate::error::Result;
use crate::token::Token;

pub struct Parser {
    #[allow(dead_code)]
    tokens: Vec<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens }
    }

    pub fn parse(&mut self) -> Result<Program> {
        todo!("parser not yet implemented")
    }
}
