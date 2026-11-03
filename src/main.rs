use std::io::Result;

use crate::lexer::Lexer;

mod lexer;

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: pico <file>");
    let source = std::fs::read_to_string(&path)?;
    for token in Lexer::new(&source) {
        println!("{:?}", token);
    }

    Ok(())
}
