pub mod c_backend;

use crate::ast::Program;
use crate::error::Result;

pub trait Backend {
    fn name(&self) -> &'static str;
    fn emit(&self, program: &Program) -> Result<String>;
}
