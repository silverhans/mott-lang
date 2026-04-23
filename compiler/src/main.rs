use std::env;
use std::fs;
use std::process::{exit, Command};

use mott::codegen::{c_backend::CBackend, Backend};
use mott::lexer::Lexer;
use mott::parser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: mott <file.mott>");
        exit(1);
    }

    let source_path = &args[1];
    let source = fs::read_to_string(source_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {}", source_path, e);
        exit(1);
    });

    // Pipeline: source -> tokens -> AST -> C -> binary
    let tokens = match Lexer::new(&source).tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    let program = match Parser::new(tokens).parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    let c_code = match CBackend.emit(&program) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    // TODO: write c_code to temp file, invoke clang to produce binary
    // For now, just print the generated C
    print!("{}", c_code);

    let _ = Command::new("clang"); // silence unused import for now
}
