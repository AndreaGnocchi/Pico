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

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
  kind: TokenKind,
  pos: Pos,
}

#[derive(Debug, PartialEq)]
pub struct LexErrorAt {
  kind: LexError,
  pos: Pos,
}

pub struct Lexer<'a> {
  input: &'a str,
  chars: Peekable<CharIndices<'a>>,
  pos: Pos,
}

impl<'a> Lexer<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      input,
      chars: input.char_indices().peekable(),
      pos: Pos { line: 1, col: 1 },
    }
  }
  
  fn next_token(&mut self) -> Result<Token, LexErrorAt> {
    self.skip_trivia(); // Space \n \t \r and //
    
    let pos = self.pos;
    let Some((start, c)) = self.advance() else {
      return Ok(Token {
        kind: TokenKind::EOF,
        pos,
      });
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
      let Some(&(_, c)) = self.chars.peek() else {
        break;
      };
      
      match c {
        c if c.is_whitespace() => {
          self.advance();
        }
        
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

impl<'a> Iterator for Lexer<'a> {
  type Item = Result<Token, LexErrorAt>;
  fn next(&mut self) -> Option<Self::Item> {
    match self.next_token() {
      Ok(token) => {
        if token.kind == TokenKind::EOF {
          None
        } else {
          Some(Ok(token))
        }
      }
      Err(e) => Some(Err(e)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use TokenKind::*;
  
  fn lex(src: &str) -> Vec<Token> {
    Lexer::new(src)
      .collect::<Result<Vec<_>, _>>()
      .expect("unexpected lex error")
  }
  
  fn kinds(src: &str) -> Vec<TokenKind> {
    lex(src).into_iter().map(|t| t.kind).collect()
  }
  
  fn first_err(src: &str) -> LexErrorAt {
    Lexer::new(src)
      .find_map(|r| r.err())
      .expect("expected a lex error")
  }
  
  fn ident(name: &str) -> TokenKind {
    Ident(name.to_string())
  }
  
  fn pos(line: usize, col: usize) -> Pos {
    Pos { line, col }
  }
  
  #[test]
  fn empty_input_yields_no_tokens() {
    assert_eq!(kinds(""), vec![]);
  }
  
  #[test]
  fn whitespace_only_yields_no_tokens() {
    assert_eq!(kinds("  \n\t\r\n  "), vec![]);
  }
  
  #[test]
  fn next_token_returns_eof_repeatedly() {
    let mut lexer = Lexer::new("");
    assert_eq!(lexer.next_token().unwrap().kind, EOF);
    assert_eq!(lexer.next_token().unwrap().kind, EOF);
  }

  #[test]
  fn comment_is_skipped() {
    assert_eq!(kinds("// hello\n1"), vec![IntLit(1)]);
  }

  #[test]
  fn comment_at_eof_without_newline() {
    assert_eq!(kinds("1 // trailing"), vec![IntLit(1)]);
  }

  #[test]
  fn comment_only_input() {
    assert_eq!(kinds("//"), vec![]);
    assert_eq!(kinds("// just a comment"), vec![]);
  }

  #[test]
  fn consecutive_comments() {
    assert_eq!(kinds("// a\n// b\n7"), vec![IntLit(7)]);
  }

  #[test]
  fn single_slash_is_division() {
    assert_eq!(kinds("1 / 2"), vec![IntLit(1), Div, IntLit(2)]);
    assert_eq!(kinds("1/2"), vec![IntLit(1), Div, IntLit(2)]);
  }

  #[test]
  fn division_after_comment() {
    assert_eq!(kinds("1 //2\n/ 3"), vec![IntLit(1), Div, IntLit(3)]);
  }

  #[test]
  fn non_ascii_in_comment_is_fine() {
    assert_eq!(kinds("// héllo wörld\n1"), vec![IntLit(1)]);
  }

  #[test]
  fn single_char_tokens() {
    assert_eq!(
      kinds("(){},;+-*/%"),
      vec![
        LParen, RParen, LCurly, RCurly, Comma, Semi, Plus, Minus, Mult, Div, Mod
      ]
    );
  }

  #[test]
  fn one_or_two_char_operators() {
    assert_eq!(
      kinds("! != < <= = == > >="),
      vec![Not, NotEq, Less, LessEq, Assign, Eq, Greater, GreaterEq]
    );
  }

  #[test]
  fn operators_use_maximal_munch() {
    assert_eq!(kinds("==="), vec![Eq, Assign]);
    assert_eq!(kinds("<=="), vec![LessEq, Assign]);
    assert_eq!(kinds("!=="), vec![NotEq, Assign]);
    assert_eq!(kinds("!!"), vec![Not, Not]);
  }

  #[test]
  fn operators_without_spaces() {
    assert_eq!(kinds("a<=b"), vec![ident("a"), LessEq, ident("b")]);
    assert_eq!(kinds("a>=b"), vec![ident("a"), GreaterEq, ident("b")]);
  }

  #[test]
  fn logical_operators() {
    assert_eq!(kinds("&& ||"), vec![And, Or]);
    assert_eq!(
      kinds("a&&b||c"),
      vec![ident("a"), And, ident("b"), Or, ident("c")]
    );
  }
  #[test]
  fn all_keywords() {
    assert_eq!(
      kinds("let fn if else while return print true false nil"),
      vec![Let, Fn, If, Else, While, Return, Print, True, False, Nil]
    );
  }

  #[test]
  fn identifiers() {
    assert_eq!(
      kinds("x foo _bar baz_1 _ a1b2"),
      vec![
        ident("x"),
        ident("foo"),
        ident("_bar"),
        ident("baz_1"),
        ident("_"),
        ident("a1b2"),
      ]
    );
  }

  #[test]
  fn keyword_prefixes_are_identifiers() {
    assert_eq!(
      kinds("letter fns iffy nil2 returned printx"),
      vec![
        ident("letter"),
        ident("fns"),
        ident("iffy"),
        ident("nil2"),
        ident("returned"),
        ident("printx"),
      ]
    );
  }

  #[test]
  fn keywords_are_case_sensitive() {
    assert_eq!(
      kinds("Let FN If TRUE Nil"),
      vec![
        ident("Let"),
        ident("FN"),
        ident("If"),
        ident("TRUE"),
        ident("Nil")
      ]
    );
  }

  #[test]
  fn keyword_adjacent_to_punctuation() {
    assert_eq!(
      kinds("if(x){return;}"),
      vec![If, LParen, ident("x"), RParen, LCurly, Return, Semi, RCurly]
    );
  }

  #[test]
  fn integers() {
    assert_eq!(
      kinds("0 7 42 1000"),
      vec![IntLit(0), IntLit(7), IntLit(42), IntLit(1000)]
    );
  }

  #[test]
  fn i64_max_is_accepted() {
    assert_eq!(kinds("9223372036854775807"), vec![IntLit(i64::MAX)]);
  }

  #[test]
  fn integer_overflow_is_an_error() {
    let err = first_err("9223372036854775808");
    assert_eq!(
      err.kind,
      LexError::InvalidNumber("9223372036854775808".to_string())
    );
    assert_eq!(err.pos, pos(1, 1));
  }

  #[test]
  fn minus_is_a_separate_token() {
    assert_eq!(kinds("-5"), vec![Minus, IntLit(5)]);
    assert_eq!(kinds("1-2"), vec![IntLit(1), Minus, IntLit(2)]);
  }

  #[test]
  fn positions_on_one_line() {
    let tokens = lex("let x = 1;");
    let got: Vec<Pos> = tokens.iter().map(|t| t.pos).collect();
    assert_eq!(
      got,
      vec![pos(1, 1), pos(1, 5), pos(1, 7), pos(1, 9), pos(1, 10)]
    );
  }

  #[test]
  fn positions_across_lines() {
    let tokens = lex("let x = 1;\n  print x;");
    let got: Vec<Pos> = tokens.iter().map(|t| t.pos).collect();
    assert_eq!(
      got,
      vec![
        pos(1, 1),
        pos(1, 5),
        pos(1, 7),
        pos(1, 9),
        pos(1, 10),
        pos(2, 3),
        pos(2, 9),
        pos(2, 10),
      ]
    );
  }

  #[test]
  fn position_is_the_start_of_a_multi_char_token() {
    let tokens = lex("<= == foo 123");
    let got: Vec<Pos> = tokens.iter().map(|t| t.pos).collect();
    assert_eq!(got, vec![pos(1, 1), pos(1, 4), pos(1, 7), pos(1, 11)]);
  }

  #[test]
  fn comment_advances_the_line_counter() {
    let tokens = lex("// comment\nlet");
    assert_eq!(tokens[0].kind, Let);
    assert_eq!(tokens[0].pos, pos(2, 1));
  }

  #[test]
  fn crlf_line_endings() {
    let tokens = lex("a\r\nb");
    assert_eq!(tokens[0].pos, pos(1, 1));
    assert_eq!(tokens[1].pos, pos(2, 1));
  }

  #[test]
  fn eof_token_position() {
    let mut lexer = Lexer::new("a\n");
    lexer.next_token().unwrap();
    let eof = lexer.next_token().unwrap();
    assert_eq!(eof.kind, EOF);
    assert_eq!(eof.pos, pos(2, 1));
  }

  #[test]
  fn unexpected_character() {
    let err = first_err("let @");
    assert_eq!(err.kind, LexError::Unexpected('@'));
    assert_eq!(err.pos, pos(1, 5));
  }

  #[test]
  fn unexpected_non_ascii_character() {
    let err = first_err("é");
    assert_eq!(err.kind, LexError::Unexpected('é'));
    assert_eq!(err.pos, pos(1, 1));
  }

  #[test]
  fn lone_ampersand() {
    let err = first_err("a & b");
    assert_eq!(err.kind, LexError::Expected('&'));
    assert_eq!(err.pos, pos(1, 3));
  }

  #[test]
  fn lone_pipe() {
    let err = first_err("|");
    assert_eq!(err.kind, LexError::Expected('|'));
    assert_eq!(err.pos, pos(1, 1));
  }

  #[test]
  fn error_position_after_newlines() {
    let err = first_err("1\n2\n  #");
    assert_eq!(err.kind, LexError::Unexpected('#'));
    assert_eq!(err.pos, pos(3, 3));
  }

  #[test]
  fn lexing_continues_after_an_error() {
    let results: Vec<_> = Lexer::new("@ 1").collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].is_err());
    assert_eq!(results[1].as_ref().unwrap().kind, IntLit(1));
  }

  #[test]
  fn print_expression() {
    assert_eq!(
      kinds("print 1 + 2 * 3;"),
      vec![Print, IntLit(1), Plus, IntLit(2), Mult, IntLit(3), Semi]
    );
  }

  #[test]
  fn let_and_assignment() {
    assert_eq!(
      kinds("let x = 10;\nx = x + 1;"),
      vec![
        Let,
        ident("x"),
        Assign,
        IntLit(10),
        Semi,
        ident("x"),
        Assign,
        ident("x"),
        Plus,
        IntLit(1),
        Semi,
      ]
    );
  }

  #[test]
  fn fib_program() {
    let src = "
      // comment
      fn fib(n) {
          if n < 2 { return n; }
          return fib(n - 1) + fib(n - 2);
      }
 
      let i = 0;
      while i < 10 {
          print fib(i);
          i = i + 1;
      }
    ";

    let expected = vec![
      // fn fib(n) {
      Fn, ident("fib"), LParen, ident("n"), RParen, LCurly,
      // if n < 2 { return n; }
      If, ident("n"), Less, IntLit(2), LCurly, Return, ident("n"), Semi, RCurly,
      // return fib(n - 1) + fib(n - 2);
      Return, ident("fib"), LParen, ident("n"), Minus, IntLit(1), RParen, Plus, ident("fib"), LParen, ident("n"), Minus, IntLit(2), RParen, Semi,
      // }
      RCurly,
      // let i = 0;
      Let, ident("i"), Assign, IntLit(0), Semi,
      // while i < 10 {
      While,
      ident("i"),
      Less,
      IntLit(10),
      LCurly,
      // print fib(i);
      Print, ident("fib"), LParen, ident("i"), RParen, Semi,
      // i = i + 1;
      ident("i"), Assign, ident("i"), Plus, IntLit(1), Semi,
      // }
      RCurly,
    ];

    assert_eq!(kinds(src), expected);
  }
}
