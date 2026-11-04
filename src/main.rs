use std::process::ExitCode;

use crate::{compiler::Compiler, lexer::Lexer, parser::Parser};

mod compiler;
mod lexer;
mod opcode;
mod parser;

fn main() -> ExitCode {
  let path = match std::env::args().nth(1) {
    Some(p) => p,
    None => {
      eprintln!("usage: pico <file>");
      return ExitCode::from(1);
    }
  };

  let source = match std::fs::read_to_string(&path) {
    Ok(s) => s,
    Err(err) => {
      eprintln!("Failed to read file '{}': {}", path, err);
      return ExitCode::from(2);
    }
  };
  
  let tokens = match Lexer::new(&source).collect::<Result<Vec<_>, _>>() {
    Ok(tokens) => tokens,
    Err(err) => {
      eprintln!("Lexer error: {:?}", err);
      return ExitCode::FAILURE;
    }
  };

  let mut parser = Parser::new(tokens);

  let mut statements = Vec::new();
  while !parser.is_at_end() {
    match parser.parse_stmt() {
      Ok(stmt) => statements.push(stmt),
      Err(err) => {
        eprintln!("Parse error: {:?}", err);
        return ExitCode::FAILURE;
      }
    }
  }

  let mut compiler = Compiler::new();
  let program = match compiler.compile(&statements) {
    Ok(ops) => ops,
    Err(err) => {
      eprintln!("Compiler error: {:?}", err);
      return ExitCode::FAILURE;
    }
  };

  for opcode in program.code {
    println!("{:?}", opcode);
  }

  ExitCode::SUCCESS
}
