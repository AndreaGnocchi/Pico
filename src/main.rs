use std::io::Result;

use crate::lexer::Lexer;

mod lexer;

fn main() -> Result<()> {
  let path = "src.pico"; // hardcode just for now to not have errors
  let src_code = std::fs::read_to_string(path)?;
  let lexer = Lexer::new(&src_code);
  Ok(())
}
