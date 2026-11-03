use crate::lexer::{Token, TokenKind};

#[derive(Debug)]
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

#[derive(Debug)]
pub enum UnaryOp {
  Neg,       // -
  Not,       // !
}

#[derive(Debug)]
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

#[derive(Debug)]
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
    while !self.check(&TokenKind::RParen) {
      let ident_token = self.peek().ok_or(ParseError::UnexpectedEOF)?.clone();
      let param_name = match ident_token.kind {
        TokenKind::Ident(param_str) => {
          self.advance();
          param_str.clone()
        }
        _ => return Err(ParseError::UnexpectedToken(ident_token)),
      };
      params.push(param_name);

      if self.check(&TokenKind::Comma) {
        self.advance();
      } else if !self.check(&TokenKind::RParen) {
        let unexpected = self.peek().cloned().ok_or(ParseError::UnexpectedEOF)?;
        return Err(ParseError::UnexpectedToken(unexpected));
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
