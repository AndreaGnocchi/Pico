use std::fmt;

use crate::lexer::{Token, TokenKind};

#[derive(Debug, PartialEq, Eq)]
pub enum BinaryOp {
  Add,       // +
  Sub,       // -
  Mul,       // *
  Div,       // /
  Mod,       // %
  Eq,        // ==
  NotEq,     // !=
  Less,      // <
  LessEq,    // <=
  Greater,   // >
  GreaterEq, // >=
  And,       // &&
  Or,        // ||
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnaryOp {
  Neg,       // -
  Not,       // !
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
  Int(i64),
  Bool(bool),
  Nil,
  Ident(String),
  Unary {
    op: UnaryOp,
    right: Box<Expr>,
  },
  Binary {
    left: Box<Expr>,
    op: BinaryOp,
    right: Box<Expr>,
  },
  Call {
    callee: Box<Expr>,
    args: Vec<Expr>,
  },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Stmt {
  Let {
    name: String,
    init: Expr,
  },
  Assign {
    name: String,
    value: Expr,
  },
  Print {
    expr: Expr,
  },
  Return {
    value: Option<Expr>,
  },
  If {
    condition: Expr,
    then_branch: Vec<Stmt>,
    else_branch: Option<Vec<Stmt>>,
  },
  While {
    condition: Expr,
    body: Vec<Stmt>,
  },
  Fn {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
  },
  Expr(Expr),
} 

#[derive(Debug)]
pub enum ParseError {
  UnexpectedEOF,
  UnexpectedToken(Token),
  TooManyArguments(Option<Token>),
}

pub struct Parser {
  tokens: Vec<Token>,
  current: usize,
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ParseError::UnexpectedEOF => write!(f, "unexpected end of input"),
      ParseError::UnexpectedToken(tok) => {
        write!(f, "unexpected token {:?} at {}", tok.kind, tok.pos)
      }
      ParseError::TooManyArguments(Some(tok)) => {
        write!(f, "too many arguments (near {:?} at {})", tok.kind, tok.pos)
      }
      ParseError::TooManyArguments(None) => write!(f, "too many arguments"),
    }
  }
}

impl Parser {
  pub fn new(tokens: Vec<Token>) -> Self {
    Self {tokens, current: 0}
  }

/*
 *   stmt = "let" IDENT "=" expr ";"
 *        | IDENT "=" expr ";"
 *        | "print" expr ";"
 *        | "return" [ expr ] ";"
 *        | "if" expr block [ "else" ( block | ifstmt ) ]
 *        | "while" expr block
 *        | "fn" IDENT "(" [ IDENT { "," IDENT } ] ")" block
 *        | expr ";" ;
 */
  
  pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
    let token = self.peek().ok_or(ParseError::UnexpectedEOF)?;

    if let TokenKind::Ident(_) = &token.kind {
      if let Some(next_token) = self.tokens.get(self.current + 1) {
        if next_token.kind == TokenKind::Assign {
          return self.parse_assign();
        }
      }
    }
    
    match token.kind {
      TokenKind::Let => self.parse_let(),
      TokenKind::Print => self.parse_print(),
      TokenKind::Return => self.parse_return(),
      TokenKind::If => self.parse_if(),
      TokenKind::While => self.parse_while(),
      TokenKind::Fn => self.parse_fn(),
      _ => {
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semi)?;
        Ok(Stmt::Expr(expr))
      }
    }
  }

  // "let" IDENT "=" expr ";"
  fn parse_let(&mut self) -> Result<Stmt, ParseError> {
    // Consume let
    self.expect(TokenKind::Let)?; 

    // Consume variable name
    let ident_token = self.peek().ok_or(ParseError::UnexpectedEOF)?.clone();
    let name = match &ident_token.kind {
      TokenKind::Ident(name_str) => {
        self.advance();
        name_str.clone()
      }
      _ => return Err(ParseError::UnexpectedToken(ident_token)),
    };

    // Consume =
    self.expect(TokenKind::Assign)?;

    // Parse assigned value
    let init = self.parse_expr()?;

    // Consume ;
    self.expect(TokenKind::Semi)?;

    Ok(Stmt::Let { name, init })
  }

  // IDENT "=" expr ";"
  fn parse_assign(&mut self) -> Result<Stmt, ParseError> {
    // Consume variable name
    let ident_token = self.peek().ok_or(ParseError::UnexpectedEOF)?.clone();
    let name = match &ident_token.kind {
      TokenKind::Ident(name_str) => {
        self.advance();
        name_str.clone()
      }
      _ => return Err(ParseError::UnexpectedToken(ident_token)),
    };

    // Consume =
    self.expect(TokenKind::Assign)?;
    
    // Parse assigned value
    let value = self.parse_expr()?;

    // Consume ;
    self.expect(TokenKind::Semi)?;
    
    Ok(Stmt::Assign { name, value })
  }

  // "print" expr ";"
  fn parse_print(&mut self) -> Result<Stmt, ParseError> {
    // Consume print
    self.expect(TokenKind::Print)?;

    // Parse expr that needs to be printed
    let expr = self.parse_expr()?;

    // Consume ;
    self.expect(TokenKind::Semi)?;
    
    Ok(Stmt::Print { expr })
  }

  // "return" [ expr ] ";"
  fn parse_return(&mut self) -> Result<Stmt, ParseError> {
    // Consume return
    self.expect(TokenKind::Return)?;

    // Parse the expr only if it is present
    let value = if self.check(&TokenKind::Semi) { None } else { Some(self.parse_expr()?) };
    
    // Consume ;
    self.expect(TokenKind::Semi)?;

    Ok(Stmt::Return { value })
  }

  // "if" expr block [ "else" ( block | ifstmt ) ]
  fn parse_if(&mut self) -> Result<Stmt, ParseError> {
    // Consume if
    self.expect(TokenKind::If)?;

    // Parse condition
    let condition = self.parse_expr()?;

    // Parse then branch
    let then_branch = self.parse_block()?;

    // Parse 'else' branch
    let else_branch = if self.match_token(&[TokenKind::Else]) {
      if self.check(&TokenKind::If) {
        // Handle nested 'if'
        let nested_if = self.parse_if()?;
        Some(vec![nested_if])
      } else {
        // Handle regular 'else' block
        Some(self.parse_block()?)
      }
    } else {
      None
    };
    
    Ok(Stmt::If { condition, then_branch, else_branch })
  }

  // "while" expr block
  fn parse_while(&mut self) -> Result<Stmt, ParseError> {
    // Consume while
    self.expect(TokenKind::While)?;

    // Parse condition
    let condition = self.parse_expr()?;

    // Parse body instruction
    let body = self.parse_block()?;

    Ok(Stmt::While { condition, body })
  }

  // "fn" IDENT "(" [ IDENT { "," IDENT } ] ")" block
  fn parse_fn(&mut self) -> Result<Stmt, ParseError> {
    // Consume fn
    self.expect(TokenKind::Fn)?;

    // Consume func name
    let ident_token = self.peek().ok_or(ParseError::UnexpectedEOF)?.clone();
    let name = match &ident_token.kind {
      TokenKind::Ident(name_str) => {
        self.advance();
        name_str.clone()
      }
      _ => return Err(ParseError::UnexpectedToken(ident_token)),
    };

    // Consume (
    self.expect(TokenKind::LParen)?;

    // Consume params
    let mut params = Vec::new();
    if !self.check(&TokenKind::RParen) {
      loop {
        let ident_token = self.peek().ok_or(ParseError::UnexpectedEOF)?.clone();
        match ident_token.kind {
          TokenKind::Ident(param_str) => {
            self.advance();
            params.push(param_str);
          }
          _ => return Err(ParseError::UnexpectedToken(ident_token)),
        }
 
        if !self.match_token(&[TokenKind::Comma]) {
          break;
        }
      }
    }
    
    // Consume )
    self.expect(TokenKind::RParen)?;

    // Parse function body
    let body = self.parse_block()?;
    
    Ok(Stmt::Fn {name, body, params})
  }

  // block = "{" { stmt } "}" ;
  fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
    // Consume {
    self.expect(TokenKind::LCurly)?;

    // Parse and consume statement
    let mut stmt_vec = Vec::new();
    while !self.check(&TokenKind::RCurly) {
      let stmt = self.parse_stmt()?;
      stmt_vec.push(stmt);
    }

    // Consume }
    self.expect(TokenKind::RCurly)?;

    Ok(stmt_vec)
  }

  // expr = or ;
  fn parse_expr(&mut self) -> Result<Expr, ParseError> {
    self.parse_or()
  }

  // or = and { "||" and } ;
  fn parse_or(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.parse_and()?;

    while self.match_token(&[TokenKind::Or]) {
      let right = self.parse_and()?;
      expr = Expr::Binary {
        left: Box::new(expr),
        op: BinaryOp::Or,
        right: Box::new(right),
      };
    }

    Ok(expr)
  }

  // and = equality { "&&" equality } ;
  fn parse_and(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.parse_equality()?;

    while self.match_token(&[TokenKind::And]) {
      let right = self.parse_equality()?;
      expr = Expr::Binary { left: Box::new(expr), op: BinaryOp::And, right: Box::new(right) };
    }

    Ok(expr)
  }

  // equality = compare { ("==" | "!=") compare } ;
  fn parse_equality(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.parse_compare()?;

    while self.match_token(&[TokenKind::Eq, TokenKind::NotEq]) {
      let op = match self.previous().unwrap().kind {
        TokenKind::Eq => BinaryOp::Eq,
        TokenKind::NotEq => BinaryOp::NotEq,
        _ => unreachable!(),
      };
      
      let right = self.parse_compare()?;
      expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
    }

    Ok(expr)
  }

  // compare = term { ("<" | "<=" | ">" | ">=") term } ;
  fn parse_compare(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.parse_term()?;

    while self.match_token(&[
      TokenKind::Less,
      TokenKind::LessEq,
      TokenKind::Greater,
      TokenKind::GreaterEq,
    ]) {
      let op = match self.previous().unwrap().kind {
        TokenKind::Less => BinaryOp::Less,
        TokenKind::LessEq => BinaryOp::LessEq,
        TokenKind::Greater => BinaryOp::Greater,
        TokenKind::GreaterEq => BinaryOp::GreaterEq,
        _ => unreachable!(),
      };
      
      let right = self.parse_term()?;
      expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
    }
    
    Ok(expr)
  }

  // term = factor { ("+" | "-") factor } ;
  fn parse_term(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.parse_factor()?;

    while self.match_token(&[TokenKind::Plus, TokenKind::Minus]) {
      let op = match self.previous().unwrap().kind {
        TokenKind::Plus => BinaryOp::Add,
        TokenKind::Minus => BinaryOp::Sub,
        _ => unreachable!(),
      };

      let right = self.parse_factor()?;
      expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
    }
    
    Ok(expr)
  }

  // factor = unary { ("*" | "/" | "%") unary } ;
  fn parse_factor(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.parse_unary()?;

    while self.match_token(&[TokenKind::Mult, TokenKind::Div, TokenKind::Mod]) {
      let op = match self.previous().unwrap().kind {
        TokenKind::Mult => BinaryOp::Mul,
        TokenKind::Div => BinaryOp::Div,
        TokenKind::Mod => BinaryOp::Mod,
        _ => unreachable!(),
      };

      let right = self.parse_unary()?;
      expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
    }
    
    Ok(expr)
  }

  // unary = ("-" | "!") unary | call ;
  fn parse_unary(&mut self) -> Result<Expr, ParseError> {
    if self.match_token(&[TokenKind::Minus, TokenKind::Not]) {
      let op = match self.previous().unwrap().kind {
        TokenKind::Minus => UnaryOp::Neg,
        TokenKind::Not => UnaryOp::Not,
        _ => unreachable!(),
      };
      
      let right = self.parse_unary()?;
      return Ok(Expr::Unary { op, right: Box::new(right) });
    }

    self.parse_call()
  }

  // call = primary { "(" [ expr { "," expr } ] ")" } ;
  fn parse_call(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.parse_primary()?;

    while self.match_token(&[TokenKind::LParen]) {
        expr = self.finish_call(expr)?;
    }
    
    Ok(expr)
  }

  fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
    let mut args = Vec::new();

    // Consume args
    if !self.check(&TokenKind::RParen) {
      loop {
        if args.len() >= 255 {
          return Err(ParseError::TooManyArguments(self.peek().cloned()));
        }
        
        args.push(self.parse_expr()?);
        if !self.match_token(&[TokenKind::Comma]) {
          if !self.check(&TokenKind::RParen) {
            let unexpected = self.peek().cloned().ok_or(ParseError::UnexpectedEOF)?;
            return Err(ParseError::UnexpectedToken(unexpected));
          } else {
            break;
          }
        }
      }
    }
      
    // Consume )
    self.expect(TokenKind::RParen)?;
    
    Ok(Expr::Call { callee: Box::new(callee), args })
  }

  // primary = INT | "true" | "false" | "nil" | IDENT | "(" expr ")" ;
  fn parse_primary(&mut self) -> Result<Expr, ParseError> {
    let token = self.peek().ok_or(ParseError::UnexpectedEOF)?.clone();

    match token.kind {
      TokenKind::True => {
        self.advance();
        Ok(Expr::Bool(true))
      }
      TokenKind::False => {
        self.advance();
        Ok(Expr::Bool(false))
      }
      TokenKind::Nil => {
        self.advance();
        Ok(Expr::Nil)
      }
      TokenKind::IntLit(val) => {
        self.advance();
        Ok(Expr::Int(val))
      }
      TokenKind::Ident(name) => {
        self.advance();
        Ok(Expr::Ident(name))
      }
      TokenKind::LParen => {
        self.advance();
        let expr = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(expr)
      }
      _ => Err(ParseError::UnexpectedToken(token)),
    }
  }
  
  fn peek(&self) -> Option<&Token> {
    self.tokens.get(self.current)
  }

  fn expect(&mut self, expected: TokenKind) -> Result<Token, ParseError> {
    let token = self.peek().ok_or(ParseError::UnexpectedEOF)?.clone();
    
    if token.kind == expected {
      self.advance();
      Ok(token)
    } else {
      Err(ParseError::UnexpectedToken(token))
    }
  }
  
  fn previous(&self) -> Option<&Token> {
    if self.current == 0 {
      None
    } else {
      self.tokens.get(self.current - 1)
    }
  }

  pub fn is_at_end(&self) -> bool {
    match self.peek() {
      Some(token) => token.kind == TokenKind::EOF,
      None => true,
    }
  }

  fn advance(&mut self) -> Option<Token> {
    if !self.is_at_end() {
      self.current += 1;
    }
    self.previous().cloned()
  }

  fn check(&self, kind: &TokenKind) -> bool {
    match self.peek() {
      Some(token) => &token.kind == kind,
      None => false,
    }
  }

  fn match_token(&mut self, kinds: &[TokenKind]) -> bool {
    for kind in kinds {
      if self.check(kind) {
        self.advance();
        return true;
      }
    }
    false
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lexer::Lexer;
 
  fn parser_for(src: &str) -> Parser {
    let tokens = Lexer::new(src)
      .collect::<Result<Vec<Token>, _>>()
      .expect("test source should lex");
    Parser::new(tokens)
  }
 
  fn require_end<T>(parser: &Parser, value: T) -> Result<T, ParseError> {
    if parser.is_at_end() {
      Ok(value)
    } else {
      Err(ParseError::UnexpectedToken(parser.peek().unwrap().clone()))
    }
  }
 
  fn parse_expr(src: &str) -> Result<Expr, ParseError> {
    let mut parser = parser_for(src);
    let expr = parser.parse_expr()?;
    require_end(&parser, expr)
  }
 
  fn parse_stmt(src: &str) -> Result<Stmt, ParseError> {
    let mut parser = parser_for(src);
    let stmt = parser.parse_stmt()?;
    require_end(&parser, stmt)
  }
 
  fn parse_program(src: &str) -> Result<Vec<Stmt>, ParseError> {
    let mut parser = parser_for(src);
    let mut stmts = Vec::new();
    while !parser.is_at_end() {
      stmts.push(parser.parse_stmt()?);
    }
    Ok(stmts)
  }
 
  fn int(n: i64) -> Expr { Expr::Int(n) }
  fn id(name: &str) -> Expr { Expr::Ident(name.to_string()) }
  fn bin(left: Expr, op: BinaryOp, right: Expr) -> Expr {
    Expr::Binary { left: Box::new(left), op, right: Box::new(right) }
  }
  fn un(op: UnaryOp, right: Expr) -> Expr {
    Expr::Unary { op, right: Box::new(right) }
  }
  fn call(callee: Expr, args: Vec<Expr>) -> Expr {
    Expr::Call { callee: Box::new(callee), args }
  }
  fn print(expr: Expr) -> Stmt { Stmt::Print { expr } }
 
  fn call_with_n_args(n: usize) -> String {
    let args: Vec<String> = (0..n).map(|i| i.to_string()).collect();
    format!("f({})", args.join(", "))
  }
 
  #[test]
  fn literals() {
    assert_eq!(parse_expr("42").unwrap(), int(42));
    assert_eq!(parse_expr("true").unwrap(), Expr::Bool(true));
    assert_eq!(parse_expr("false").unwrap(), Expr::Bool(false));
    assert_eq!(parse_expr("nil").unwrap(), Expr::Nil);
    assert_eq!(parse_expr("x").unwrap(), id("x"));
  }
 
  #[test]
  fn parentheses_override_precedence() {
    assert_eq!(
      parse_expr("(1 + 2) * 3").unwrap(),
      bin(bin(int(1), BinaryOp::Add, int(2)), BinaryOp::Mul, int(3)),
    );
  }
 
  #[test]
  fn deeply_nested_parentheses() {
    assert_eq!(parse_expr("((((((1))))))").unwrap(), int(1));
  }
 
  #[test]
  fn mul_binds_tighter_than_add() {
    assert_eq!(
      parse_expr("1 + 2 * 3").unwrap(),
      bin(int(1), BinaryOp::Add, bin(int(2), BinaryOp::Mul, int(3))),
    );
  }
 
  #[test]
  fn subtraction_is_left_associative() {
    assert_eq!(
      parse_expr("1 - 2 - 3").unwrap(),
      bin(bin(int(1), BinaryOp::Sub, int(2)), BinaryOp::Sub, int(3)),
    );
  }
 
  #[test]
  fn division_and_mod_are_left_associative() {
    assert_eq!(
      parse_expr("8 / 4 / 2").unwrap(),
      bin(bin(int(8), BinaryOp::Div, int(4)), BinaryOp::Div, int(2)),
    );
    assert_eq!(
      parse_expr("a % b * c").unwrap(),
      bin(bin(id("a"), BinaryOp::Mod, id("b")), BinaryOp::Mul, id("c")),
    );
  }
 
  #[test]
  fn arithmetic_binds_tighter_than_comparison() {
    assert_eq!(
      parse_expr("a < b + c").unwrap(),
      bin(id("a"), BinaryOp::Less, bin(id("b"), BinaryOp::Add, id("c"))),
    );
  }
 
  #[test]
  fn comparison_binds_tighter_than_equality() {
    assert_eq!(
      parse_expr("a == b < c").unwrap(),
      bin(id("a"), BinaryOp::Eq, bin(id("b"), BinaryOp::Less, id("c"))),
    );
  }
 
  #[test]
  fn and_binds_tighter_than_or() {
    assert_eq!(
      parse_expr("a || b && c").unwrap(),
      bin(id("a"), BinaryOp::Or, bin(id("b"), BinaryOp::And, id("c"))),
    );
  }
 
  #[test]
  fn equality_binds_tighter_than_and() {
    assert_eq!(
      parse_expr("a && b == c").unwrap(),
      bin(id("a"), BinaryOp::And, bin(id("b"), BinaryOp::Eq, id("c"))),
    );
  }
 
  #[test]
  fn logical_operators_are_left_associative() {
    assert_eq!(
      parse_expr("a || b || c").unwrap(),
      bin(bin(id("a"), BinaryOp::Or, id("b")), BinaryOp::Or, id("c")),
    );
  }
 
  #[test]
  fn unary_binds_tighter_than_binary() {
    assert_eq!(
      parse_expr("-a + b").unwrap(),
      bin(un(UnaryOp::Neg, id("a")), BinaryOp::Add, id("b")),
    );
    assert_eq!(
      parse_expr("-a * b").unwrap(),
      bin(un(UnaryOp::Neg, id("a")), BinaryOp::Mul, id("b")),
    );
    assert_eq!(
      parse_expr("!a == b").unwrap(),
      bin(un(UnaryOp::Not, id("a")), BinaryOp::Eq, id("b")),
    );
  }
 
  #[test]
  fn unary_operators_nest() {
    assert_eq!(
      parse_expr("!!x").unwrap(),
      un(UnaryOp::Not, un(UnaryOp::Not, id("x"))),
    );
    assert_eq!(
      parse_expr("- -x").unwrap(),
      un(UnaryOp::Neg, un(UnaryOp::Neg, id("x"))),
    );
  }
 
  #[test]
  fn call_binds_tighter_than_unary() {
    assert_eq!(
      parse_expr("-f(1)").unwrap(),
      un(UnaryOp::Neg, call(id("f"), vec![int(1)])),
    );
  }
 
  #[test]
  fn call_with_no_arguments() {
    assert_eq!(parse_expr("f()").unwrap(), call(id("f"), vec![]));
  }
 
  #[test]
  fn call_with_several_arguments() {
    assert_eq!(
      parse_expr("f(1, 2)").unwrap(),
      call(id("f"), vec![int(1), int(2)]),
    );
  }
 
  #[test]
  fn call_arguments_are_full_expressions() {
    assert_eq!(
      parse_expr("f(1 + 2, g())").unwrap(),
      call(
        id("f"),
        vec![bin(int(1), BinaryOp::Add, int(2)), call(id("g"), vec![])],
      ),
    );
  }
 
  #[test]
  fn nested_calls() {
    assert_eq!(
      parse_expr("f(g(1))").unwrap(),
      call(id("f"), vec![call(id("g"), vec![int(1)])]),
    );
  }
 
  #[test]
  fn chained_calls() {
    assert_eq!(
      parse_expr("f(1)(2)").unwrap(),
      call(call(id("f"), vec![int(1)]), vec![int(2)]),
    );
  }
 
  #[test]
  fn call_argument_limit_is_255() {
    match parse_expr(&call_with_n_args(255)).unwrap() {
      Expr::Call { args, .. } => assert_eq!(args.len(), 255),
      other => panic!("expected a call, got {:?}", other),
    }
    assert!(matches!(
      parse_expr(&call_with_n_args(256)),
      Err(ParseError::TooManyArguments(_))
    ));
  }
 
  #[test]
  fn malformed_expressions_are_errors() {
    for src in [
      "",
      "1 +",
      "* 2",
      "-",
      "(1 + 2",
      ")",
      "1 2",
      "f(1,)",
      "f(1",
      "f(,)",
    ] {
      assert!(parse_expr(src).is_err(), "expected an error for {src:?}");
    }
  }
 
  #[test]
  fn let_statement() {
    assert_eq!(
      parse_stmt("let x = 1 + 2;").unwrap(),
      Stmt::Let { name: "x".to_string(), init: bin(int(1), BinaryOp::Add, int(2)) },
    );
  }
 
  #[test]
  fn assign_statement() {
    assert_eq!(
      parse_stmt("x = 5;").unwrap(),
      Stmt::Assign { name: "x".to_string(), value: int(5) },
    );
  }
 
  #[test]
  fn print_statement() {
    assert_eq!(parse_stmt("print 1;").unwrap(), print(int(1)));
  }
 
  #[test]
  fn return_with_and_without_value() {
    assert_eq!(parse_stmt("return;").unwrap(), Stmt::Return { value: None });
    assert_eq!(
      parse_stmt("return 1;").unwrap(),
      Stmt::Return { value: Some(int(1)) },
    );
  }
 
  #[test]
  fn expression_statement() {
    assert_eq!(
      parse_stmt("f(1);").unwrap(),
      Stmt::Expr(call(id("f"), vec![int(1)])),
    );
  }
 
  #[test]
  fn double_equals_is_a_comparison_not_an_assignment() {
    assert_eq!(
      parse_stmt("x == 1;").unwrap(),
      Stmt::Expr(bin(id("x"), BinaryOp::Eq, int(1))),
    );
  }
 
  #[test]
  fn if_without_else() {
    assert_eq!(
      parse_stmt("if x { print 1; }").unwrap(),
      Stmt::If {
        condition: id("x"),
        then_branch: vec![print(int(1))],
        else_branch: None,
      },
    );
  }
 
  #[test]
  fn if_else() {
    assert_eq!(
      parse_stmt("if x { print 1; } else { print 2; }").unwrap(),
      Stmt::If {
        condition: id("x"),
        then_branch: vec![print(int(1))],
        else_branch: Some(vec![print(int(2))]),
      },
    );
  }
 
  #[test]
  fn else_if_chain_nests_an_if_in_the_else_branch() {
    assert_eq!(
      parse_stmt("if a { print 1; } else if b { print 2; } else { print 3; }").unwrap(),
      Stmt::If {
        condition: id("a"),
        then_branch: vec![print(int(1))],
        else_branch: Some(vec![Stmt::If {
          condition: id("b"),
          then_branch: vec![print(int(2))],
          else_branch: Some(vec![print(int(3))]),
        }]),
      },
    );
  }
 
  #[test]
  fn while_statement() {
    assert_eq!(
      parse_stmt("while i < 3 { i = i + 1; }").unwrap(),
      Stmt::While {
        condition: bin(id("i"), BinaryOp::Less, int(3)),
        body: vec![Stmt::Assign {
          name: "i".to_string(),
          value: bin(id("i"), BinaryOp::Add, int(1)),
        }],
      },
    );
  }
 
  #[test]
  fn fn_with_zero_params_and_empty_body() {
    assert_eq!(
      parse_stmt("fn foo() {}").unwrap(),
      Stmt::Fn { name: "foo".to_string(), params: vec![], body: vec![] },
    );
  }
 
  #[test]
  fn fn_with_several_params() {
    assert_eq!(
      parse_stmt("fn add(a, b, c) { return a + b + c; }").unwrap(),
      Stmt::Fn {
        name: "add".to_string(),
        params: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        body: vec![Stmt::Return {
          value: Some(bin(bin(id("a"), BinaryOp::Add, id("b")), BinaryOp::Add, id("c"))),
        }],
      },
    );
  }
 
  #[test]
  fn malformed_statements_are_errors() {
    for src in [
      "let x = 5",
      "print 1",
      "return 1",
      "f(1)",
      "let = 5;",
      "let x 5;",
      "let x = ;",
      "x = ;",
      "x = y = 1;",
      "1 = 2;",
      "print;",
      "return",
      "fn f(,) {}",
      "fn f(a,) {}",
      "fn f(a b) {}",
      "fn (a) {}",
      "fn f(a)",
      "fn f(",
      "if",
      "if x",
      "if x { print 1; } else",
      "else { print 1; }",
      "while",
      "if x {",
      "while true { print 1;",
      "fn f() { let x = 5;",
      "}",
      "let x =",
    ] {
      assert!(parse_stmt(src).is_err(), "expected an error for {src:?}");
    }
  }
 
  #[test]
  fn stray_closing_brace_after_valid_code_is_an_error() {
    assert!(parse_program("print 1; }").is_err());
  }
 
  #[test]
  fn empty_program() {
    assert_eq!(parse_program("").unwrap(), vec![]);
    assert_eq!(parse_program("   \n  ").unwrap(), vec![]);
  }
 
  #[test]
  fn design_doc_example_program() {
    let src = "
      let x = 10;
      x = x + 1;
 
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
 
    let fib_call = |delta: i64| call(id("fib"), vec![bin(id("n"), BinaryOp::Sub, int(delta))]);
 
    let expected = vec![
      Stmt::Let { name: "x".to_string(), init: int(10) },
      Stmt::Assign { name: "x".to_string(), value: bin(id("x"), BinaryOp::Add, int(1)) },
      Stmt::Fn {
        name: "fib".to_string(),
        params: vec!["n".to_string()],
        body: vec![
          Stmt::If {
            condition: bin(id("n"), BinaryOp::Less, int(2)),
            then_branch: vec![Stmt::Return { value: Some(id("n")) }],
            else_branch: None,
          },
          Stmt::Return {
            value: Some(bin(fib_call(1), BinaryOp::Add, fib_call(2))),
          },
        ],
      },
      Stmt::Let { name: "i".to_string(), init: int(0) },
      Stmt::While {
        condition: bin(id("i"), BinaryOp::Less, int(10)),
        body: vec![
          print(call(id("fib"), vec![id("i")])),
          Stmt::Assign { name: "i".to_string(), value: bin(id("i"), BinaryOp::Add, int(1)) },
        ],
      },
    ];
 
    assert_eq!(parse_program(src).unwrap(), expected);
  }
}
