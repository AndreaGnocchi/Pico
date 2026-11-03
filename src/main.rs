use std::process::ExitCode;

use crate::{lexer::Lexer, parser::Parser};

mod lexer;
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

  for stmt in statements {
    println!("{:?}", stmt);
  }

  ExitCode::SUCCESS
}
