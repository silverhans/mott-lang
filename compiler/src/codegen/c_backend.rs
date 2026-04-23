use crate::ast::Program;
use crate::codegen::Backend;
use crate::error::Result;

pub struct CBackend;

impl Backend for CBackend {
    fn name(&self) -> &'static str {
        "c"
    }

    fn emit(&self, _program: &Program) -> Result<String> {
        todo!("C backend not yet implemented")
    }
}
