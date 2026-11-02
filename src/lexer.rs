use std::{iter::Peekable, str::CharIndices};

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
  Let,
  Fn,
  If,
  Else,
  While,
  Return,
  Print,
  True,
  False,
  Nil,
  IntLit(i64),
  Ident(String),
  LParen,
  RParen,
  LCurly,
  RCurly,
  Comma,
  Semi,
  Plus,
  Minus,
  Mult,
  Div,
  Mod,
  Not,
  Assign,
  Eq,
  NotEq,
  Less,
  LessEq,
  Greater,
  GreaterEq,
  And,
  Or,
  EOF,
}

#[derive(Debug)]
enum LexError {
  Unexpected(char),
  Expected(char),
  InvalidNumber(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos {
  line: usize,
  col: usize,
}

pub struct Token {
  kind: TokenKind,
  pos: Pos,
}

pub struct LexErrorAt {
  kind: LexError,
  pos: Pos,
}

pub struct Lexer<'a> {
  input: &'a str,
  chars: Peekable<CharIndices<'a>>,
  pos: Pos,
}

impl <'a> Lexer<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      input,
      chars: input.char_indices().peekable(),
      pos: Pos { line: 1, col: 1 },
    }
  }

  pub fn next_token(&mut self) -> Result<Token, LexErrorAt> {
    self.skip_trivia(); // Space \n \t \r and //

    let pos = self.pos;
    let Some((start, c)) = self.advance() else {
      return Ok(Token { kind: TokenKind::EOF, pos });
    };

    let kind = match c {
      // Single char
      '(' => TokenKind::LParen,
      ')' => TokenKind::RParen,
      '{' => TokenKind::LCurly,
      '}' => TokenKind::RCurly,
      ',' => TokenKind::Comma,
      ';' => TokenKind::Semi,
      '+' => TokenKind::Plus,
      '-' => TokenKind::Minus,
      '*' => TokenKind::Mult,
      '/' => TokenKind::Div,
      '%' => TokenKind::Mod,

      // 1 or 2 chars
      '!' => self.either('=', TokenKind::NotEq, TokenKind::Not),
      '<' => self.either('=', TokenKind::LessEq, TokenKind::Less),
      '=' => self.either('=', TokenKind::Eq, TokenKind::Assign),
      '>' => self.either('=', TokenKind::GreaterEq, TokenKind::Greater),

      // 2 chars
      '&' => {
        self.expect('&', pos)?;
        TokenKind::And
      }
      '|' => {
        self.expect('|', pos)?;
        TokenKind::Or
      }
      
      // Numbers
      '0'..='9' => self.number(start, pos)?,

      // Ident or Keyword
      c if c == '_' || c.is_ascii_alphabetic() => self.ident_or_keyword(start),

      other => return Err(Self::error(LexError::Unexpected(other), pos)),
    };
    
    Ok(Token { kind, pos })
  }

  fn advance(&mut self) -> Option<(usize, char)> {
    let (i, c) = self.chars.next()?;
    if c == '\n' {
      self.pos.line += 1;
      self.pos.col = 1;
    } else {
      self.pos.col += 1;
    }
    Some((i, c))
  }
  
  fn skip_trivia(&mut self) {
    loop {
      let Some(&(_, c)) = self.chars.peek() else { break; };
      
      match c {
        c if c.is_whitespace() => { self.advance(); }
        
        '/' => {
          let mut peakable = self.chars.clone();
          peakable.next();
          if let Some((_, '/')) = peakable.next() {
            while let Some(&(_, ch)) = self.chars.peek() {
              self.advance();
              if ch == '\n' {
                break;
              }
            }
          } else {
            break;
          }
        }
        
        _ => break,
      }
    }
  }

  fn eat(&mut self, expected: char) -> bool {
    match self.chars.peek() {
      Some(&(_, c)) if c == expected => {
        self.advance();
        true
      }
      _ => false,
    }
  }

  fn either(&mut self, next: char, opt1: TokenKind, opt2: TokenKind) -> TokenKind {
    if self.eat(next) { opt1 } else { opt2 }
  }

  fn expect(&mut self, next: char, pos: Pos) -> Result<(), LexErrorAt> {
    if self.eat(next) {
      Ok(())
    } else {
      Err(Self::error(LexError::Expected(next), pos))
    }
  }

  fn number(&mut self, start: usize, pos: Pos) -> Result<TokenKind, LexErrorAt> {
    let mut end = start + 1;
    while let Some(&(i, c)) = self.chars.peek() {
      if c.is_ascii_digit() {
        end = i + c.len_utf8();
        self.advance();
      } else {
        break;
      }
    }

    let slice = &self.input[start..end];
    match slice.parse::<i64>() {
      Ok(n) => Ok(TokenKind::IntLit(n)),
      Err(_) => Err(Self::error(LexError::InvalidNumber(slice.to_string()), pos)),
    }
  }
  
  fn ident_or_keyword(&mut self, start: usize) -> TokenKind {
    let mut end = start + 1;
    while let Some(&(i, c)) = self.chars.peek() {
      if c == '_' || c.is_ascii_alphanumeric() {
        end = i + c.len_utf8();
        self.advance();
      } else {
        break;
      }
    }
    
    match &self.input[start..end] {
      "let" => TokenKind::Let,
      "fn" => TokenKind::Fn,
      "if" => TokenKind::If,
      "else" => TokenKind::Else,
      "while" => TokenKind::While,
      "return" => TokenKind::Return,
      "print" => TokenKind::Print,
      "true" => TokenKind::True,
      "false" => TokenKind::False,
      "nil" => TokenKind::Nil,
      name => TokenKind::Ident(name.to_string()),
    }
  }

  fn error(kind: LexError, pos: Pos) -> LexErrorAt {
    LexErrorAt { kind, pos }
  }
}
