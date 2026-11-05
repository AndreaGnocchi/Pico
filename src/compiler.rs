use std::collections::{HashMap};

use crate::{opcode::{Opcode, Value}, parser::{BinaryOp, Expr, Stmt, UnaryOp}};

#[derive(Debug)]
pub enum CompilerError {
  NonJumpOpcode(String),
  AlreadyExistingVariable(String),
  FunctionNotAtTopLevel(String),
  ReturnOutsideFunction,
}
 
#[derive(Clone, Debug)]
struct Local {
  name: String,
  depth: usize,
}
 
#[derive(Clone, Debug)]
struct FnCtx {
  locals: Vec<Local>,
  depth: usize,
}
 
impl FnCtx {
  fn new() -> Self {
    Self { locals: Vec::new(), depth: 0 }
  }
}

#[derive(Debug)]
pub struct Program {
  pub code: Vec<Opcode>,
  pub constants: Vec<Value>,
  pub globals: usize,
}


#[derive(Clone, Debug)]
pub struct Compiler {
  opcodes: Vec<Opcode>,
  constants: Vec<Value>,
  globals: HashMap<String, usize>,
  fns: Vec<FnCtx>,
}


impl Compiler {
  pub fn new() -> Self {
    Self {
      opcodes: Vec::new(),
      constants: Vec::new(),
      fns: vec![FnCtx::new()],
      globals: HashMap::new(),
    }
  }


  pub fn compile(mut self, statements: &[Stmt]) -> Result<Program, CompilerError> {
    for stmt in statements {
      self.compile_stmt(stmt)?;
    }
    
    Ok(Program { code: self.opcodes, constants: self.constants, globals: self.globals.len() })
  }

  fn ctx(&self) -> &FnCtx {
    self.fns.last().unwrap()
  }
  
  fn ctx_mut(&mut self) -> &mut FnCtx {
    self.fns.last_mut().unwrap()
  }
 
  fn is_global_scope(&self) -> bool {
    self.fns.len() == 1 && self.ctx().depth == 0
  }
 
  fn declare_local(&mut self, name: &str) -> Result<usize, CompilerError> {
    let ctx = self.ctx_mut();
    let depth = ctx.depth;
    if ctx.locals.iter().rev().take_while(|l| l.depth == depth).any(|l| l.name == name) {
      return Err(CompilerError::AlreadyExistingVariable(format!(
        "Variable '{}' already declared in this scope.", name)));
    }
    ctx.locals.push(Local { name: name.to_string(), depth });
    Ok(ctx.locals.len())
  }
  
  fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompilerError> {
    match stmt {
     Stmt::Let { name, init } => {
        self.compile_expr(init)?;
        if self.is_global_scope() {
          let global_idx = self.resolve_global(name);
          self.opcodes.push(Opcode::SetGlobal(global_idx));
        } else {
          self.declare_local(name)?;
        }
      }

      Stmt::Assign { name, value } => {
        self.compile_expr(value)?;
        if let Some(slot) = self.resolve_local(name) {
          self.opcodes.push(Opcode::SetLocal(slot));
        } else {
          let global_idx = self.resolve_global(name);
          self.opcodes.push(Opcode::SetGlobal(global_idx));
        }
      }
      Stmt::Print { expr } => {
        self.compile_expr(expr)?;
        self.opcodes.push(Opcode::Print);
      }
      Stmt::Expr(expr) => {
        self.compile_expr(expr)?;
        self.opcodes.push(Opcode::Pop);
      }
      Stmt::Return { value } => {
        if self.fns.len() == 1 {
          return Err(CompilerError::ReturnOutsideFunction);
        }
        match value {
          Some(value) => self.compile_expr(value)?,
          None => { self.emit(Opcode::Nil); }
        }
        self.emit(Opcode::Ret);
      }
      Stmt::If { condition, then_branch, else_branch } => {
        self.compile_expr(condition)?;
        let else_jump = self.emit(Opcode::JumpIfFalse(0));
        self.emit(Opcode::Pop);
        
        self.compile_block(then_branch)?;
        let end_jump = self.emit(Opcode::Jump(0));
        self.patch_jump(else_jump)?;
        self.emit(Opcode::Pop);
        
        if let Some(else_stmts) = else_branch {
          self.compile_block(else_stmts)?;
        }
        self.patch_jump(end_jump)?;
      }
      Stmt::While { condition, body } => {
        let loop_start = self.opcodes.len();
        self.compile_expr(condition)?;
        let jump_if_false = self.emit(Opcode::JumpIfFalse(0));
        self.opcodes.push(Opcode::Pop);
        
        self.compile_block(body)?;
        
        let loop_offset = (loop_start as isize) - (self.opcodes.len() as isize) - 1;
        self.opcodes.push(Opcode::Loop(loop_offset));
        
        self.patch_jump(jump_if_false)?;
        self.opcodes.push(Opcode::Pop);
      }
      Stmt::Fn { name, params, body } => {
        if !self.is_global_scope() {
          return Err(CompilerError::FunctionNotAtTopLevel(name.clone()));
        }
        let global_idx = self.resolve_global(name);
 
        let skip_jump = self.emit(Opcode::Jump(0));
        let addr = self.opcodes.len();
 
        self.fns.push(FnCtx::new());
        for param in params {
          self.declare_local(param)?;
        }
        for stmt in body {
          self.compile_stmt(stmt)?;
        }
        self.emit(Opcode::Nil);
        self.emit(Opcode::Ret);
        self.fns.pop();
 
        self.patch_jump(skip_jump)?;
 
        let const_idx = self.add_constant(Value::Function { addr, arity: params.len() });
        self.emit(Opcode::Const(const_idx));
        self.emit(Opcode::SetGlobal(global_idx));
      }
    }
    
    Ok(())
  }

  fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompilerError> {
    match expr {
      Expr::Int(val) => {
        let const_idx = self.add_constant(Value::Int(*val));
        self.opcodes.push(Opcode::Const(const_idx));
      }
      Expr::Bool(val) => {
        if *val {
          self.opcodes.push(Opcode::True);
        } else {
          self.opcodes.push(Opcode::False);
        }
      }
      Expr::Nil => self.opcodes.push(Opcode::Nil),
      Expr::Ident(name) => {
        if let Some(local_idx) = self.resolve_local(name) {
          self.opcodes.push(Opcode::GetLocal(local_idx))
        } else {
          let global_idx = self.resolve_global(name);
          self.opcodes.push(Opcode::GetGlobal(global_idx));
        }
      }
      Expr::Unary { op, right } => {
        self.compile_expr(right)?;

        match op {
          UnaryOp::Neg => self.opcodes.push(Opcode::Neg),
          UnaryOp::Not => self.opcodes.push(Opcode::Not),
        }
      }
      Expr::Binary { left, op, right } => {
        match op {
          BinaryOp::And => {
            self.compile_expr(left)?;
            let jump_if_false = self.emit(Opcode::JumpIfFalse(0));
            self.opcodes.push(Opcode::Pop);
            self.compile_expr(right)?;
            
            self.patch_jump(jump_if_false)?;
          }
          BinaryOp::Or => {
            self.compile_expr(left)?;
            let jump_if_false = self.emit(Opcode::JumpIfFalse(0));
            let jump_to_end = self.emit(Opcode::Jump(0));
            
            self.patch_jump(jump_if_false)?;
            self.opcodes.push(Opcode::Pop);
            self.compile_expr(right)?;
            
            self.patch_jump(jump_to_end)?;
          }
          _ => {
            self.compile_expr(left)?;
            self.compile_expr(right)?;

            match op {
              BinaryOp::Add => self.opcodes.push(Opcode::Add),
              BinaryOp::Sub => self.opcodes.push(Opcode::Sub),
              BinaryOp::Mul => self.opcodes.push(Opcode::Mul),
              BinaryOp::Div => self.opcodes.push(Opcode::Div),
              BinaryOp::Mod => self.opcodes.push(Opcode::Mod),
              BinaryOp::Eq => self.opcodes.push(Opcode::Eq),
              BinaryOp::NotEq => {
                self.opcodes.push(Opcode::Eq);
                self.opcodes.push(Opcode::Not);
              }
              BinaryOp::Less => self.opcodes.push(Opcode::Lt),
              BinaryOp::LessEq => self.opcodes.push(Opcode::Le),
              BinaryOp::Greater => {
                self.opcodes.push(Opcode::Le);
                self.opcodes.push(Opcode::Not);
              }
              BinaryOp::GreaterEq => {
                self.opcodes.push(Opcode::Lt);
                self.opcodes.push(Opcode::Not);
              }
              BinaryOp::And | BinaryOp::Or => unreachable!(),
            }
          }
        }
      }
      Expr::Call { callee, args } => {
        self.compile_expr(callee)?;
        for arg in args {
          self.compile_expr(arg)?;
        }
        self.opcodes.push(Opcode::Call(args.len()));
      }
    }
    
    Ok(())
  }

  fn emit(&mut self, op: Opcode) -> usize {
    self.opcodes.push(op);
    self.opcodes.len() - 1
  }

  fn add_constant(&mut self, val: Value) -> usize {
    self.constants.push(val);
    self.constants.len() - 1
  }

  fn resolve_local(&self, name: &str) -> Option<usize> {
    self.ctx().locals.iter().rposition(|l| l.name == name).map(|i| i + 1)
  }

  fn resolve_global(&mut self, name: &str) -> usize {
    if let Some(&idx) = self.globals.get(name) {
      return idx;
    }
    
    let idx = self.globals.len();
    self.globals.insert(name.to_string(), idx);
    idx
  }
  
  fn patch_jump(&mut self, offset_index: usize) -> Result<(), CompilerError>{
    let jump_target = self.opcodes.len();
    let diff = (jump_target as isize) - (offset_index as isize) - 1;

    match &mut self.opcodes[offset_index] {
      Opcode::Jump(offset) |
      Opcode::JumpIfFalse(offset) |
      Opcode::Loop(offset) => *offset = diff,
      _ => return Err(CompilerError::NonJumpOpcode(format!(
        "Tried to patch a non-jump opcode at index {}", offset_index))),
    }

    Ok(())
  }

  fn compile_block(&mut self, statements: &[Stmt]) -> Result<(), CompilerError> {
    self.ctx_mut().depth += 1;
    for stmt in statements {
      self.compile_stmt(stmt)?;
    }
    self.ctx_mut().depth -= 1;
 
 
    let depth = self.ctx().depth;
    while self.ctx().locals.last().is_some_and(|l| l.depth > depth) {
      self.ctx_mut().locals.pop();
      self.emit(Opcode::Pop);
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{lexer::Lexer, parser::Parser};

  fn compile_src(src: &str) -> Result<Program, CompilerError> {
    let tokens = Lexer::new(src)
      .collect::<Result<Vec<_>,_>>()
      .expect("test source should lex");
    let mut parser = Parser::new(tokens);
    let mut stmts = Vec::new();
    while !parser.is_at_end() {
      stmts.push(parser.parse_stmt().expect("test source should parse"));
    }
    
    Compiler::new().compile(&stmts)
  }

  fn program(src: &str) -> Program {
    compile_src(src).expect("test source should compile")
  }

  fn code(src: &str) -> Vec<Opcode> {
    program(src).code
  }

  fn binary_tail(op: &str) -> Vec<Opcode> {
    let c = code(&format!("print 1 {op} 2;"));
    c[2..c.len() - 1].to_vec()
  }

  #[test]
  fn arithmetic_respects_precedence() {
    let p = program("print 1 + 2 * 3;");
    assert_eq!(
      p.code,
      vec![
        Opcode::Const(0), Opcode::Const(1), Opcode::Const(2),
        Opcode::Mul, Opcode::Add, Opcode::Print,
      ]
    );
    assert_eq!(p.constants, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
  }
 
  #[test]
  fn parentheses_change_emission_order() {
    assert_eq!(
      code("print (1 + 2) * 3;"),
      vec![
        Opcode::Const(0), Opcode::Const(1), Opcode::Add,
        Opcode::Const(2), Opcode::Mul, Opcode::Print,
      ]
    );
  }
 
  #[test]
  fn literals() {
    assert_eq!(
      code("print true; print false; print nil;"),
      vec![
        Opcode::True, Opcode::Print,
        Opcode::False, Opcode::Print,
        Opcode::Nil, Opcode::Print,
      ]
    );
  }
 
  #[test]
  fn unary_operators() {
    assert_eq!(
      code("print -1; print !true;"),
      vec![
        Opcode::Const(0), Opcode::Neg, Opcode::Print,
        Opcode::True, Opcode::Not, Opcode::Print,
      ]
    );
  }
 
  #[test]
  fn arithmetic_operators() {
    assert_eq!(binary_tail("+"), vec![Opcode::Add]);
    assert_eq!(binary_tail("-"), vec![Opcode::Sub]);
    assert_eq!(binary_tail("*"), vec![Opcode::Mul]);
    assert_eq!(binary_tail("/"), vec![Opcode::Div]);
    assert_eq!(binary_tail("%"), vec![Opcode::Mod]);
  }
 
  #[test]
  fn comparison_operators_use_only_eq_lt_le() {
    assert_eq!(binary_tail("=="), vec![Opcode::Eq]);
    assert_eq!(binary_tail("!="), vec![Opcode::Eq, Opcode::Not]);
    assert_eq!(binary_tail("<"), vec![Opcode::Lt]);
    assert_eq!(binary_tail("<="), vec![Opcode::Le]);
    assert_eq!(binary_tail(">"), vec![Opcode::Le, Opcode::Not]);
    assert_eq!(binary_tail(">="), vec![Opcode::Lt, Opcode::Not]);
  }
 
  #[test]
  fn expression_statement_discards_its_value() {
    assert_eq!(code("1;"), vec![Opcode::Const(0), Opcode::Pop]);
  }
 
  #[test]
  fn and_short_circuits_and_keeps_left_value_when_falsy() {
    assert_eq!(
      code("print true && false;"),
      vec![
        Opcode::True,
        Opcode::JumpIfFalse(2),
        Opcode::Pop,
        Opcode::False,
        Opcode::Print,
      ]
    );
  }
 
  #[test]
  fn or_short_circuits_and_keeps_left_value_when_truthy() {
    assert_eq!(
      code("print true || false;"),
      vec![
        Opcode::True,
        Opcode::JumpIfFalse(1),
        Opcode::Jump(2),
        Opcode::Pop,
        Opcode::False,
        Opcode::Print,
      ]
    );
  }

  #[test]
  fn top_level_variables_are_globals() {
    let p = program("let x = 1; x = x + 2; print x;");
    assert_eq!(
      p.code,
      vec![
        Opcode::Const(0), Opcode::SetGlobal(0),
        Opcode::GetGlobal(0), Opcode::Const(1), Opcode::Add, Opcode::SetGlobal(0),
        Opcode::GetGlobal(0), Opcode::Print,
      ]
    );
    assert_eq!(p.globals, 1);
  }
 
  #[test]
  fn each_global_name_gets_its_own_index() {
    let p = program("let a = 1; let b = 2; print a + b;");
    assert_eq!(p.globals, 2);
    assert_eq!(
      &p.code[4..],
      &[Opcode::GetGlobal(0), Opcode::GetGlobal(1), Opcode::Add, Opcode::Print]
    );
  }
  
  #[test]
  fn if_without_else_pops_the_condition_exactly_once_on_each_path() {
    assert_eq!(
      code("if true { print 1; }"),
      vec![
        Opcode::True,
        Opcode::JumpIfFalse(4),
        Opcode::Pop,
        Opcode::Const(0),
        Opcode::Print,
        Opcode::Jump(1),
        Opcode::Pop,
      ]
    );
  }

  #[test]
  fn if_else() {
    assert_eq!(
      code("if true { print 1; } else { print 2; }"),
      vec![
        Opcode::True,
        Opcode::JumpIfFalse(4),
        Opcode::Pop,
        Opcode::Const(0),
        Opcode::Print,
        Opcode::Jump(3),
        Opcode::Pop,
        Opcode::Const(1),
        Opcode::Print,
      ]
    );
  }
 
  #[test]
  fn while_loops_back_to_the_condition() {
    assert_eq!(
      code("while true { print 1; }"),
      vec![
        Opcode::True,
        Opcode::JumpIfFalse(4),
        Opcode::Pop,
        Opcode::Const(0),
        Opcode::Print,
        Opcode::Loop(-6),
        Opcode::Pop,
      ]
    );
  }

  #[test]
  fn block_local_lives_on_the_stack_and_is_popped_at_block_end() {
    assert_eq!(
      code("if true { let a = 1; print a; }"),
      vec![
        Opcode::True,
        Opcode::JumpIfFalse(6),
        Opcode::Pop,
        Opcode::Const(0),
        Opcode::GetLocal(1),
        Opcode::Print,
        Opcode::Pop,
        Opcode::Jump(1),
        Opcode::Pop,
      ]
    );
  }

    #[test]
  fn locals_get_consecutive_slots_and_are_popped_in_pairs() {
    assert_eq!(
      code("if true { let a = 1; let b = 2; print a + b; }"),
      vec![
        Opcode::True,
        Opcode::JumpIfFalse(10),
        Opcode::Pop,
        Opcode::Const(0),
        Opcode::Const(1),
        Opcode::GetLocal(1),
        Opcode::GetLocal(2),
        Opcode::Add,
        Opcode::Print,
        Opcode::Pop,
        Opcode::Pop,
        Opcode::Jump(1),
        Opcode::Pop,
      ]
    );
  }
 
  #[test]
  fn inner_local_shadows_a_global() {
    assert_eq!(
      code("let a = 1; if true { let a = 2; print a; }"),
      vec![
        Opcode::Const(0), Opcode::SetGlobal(0),
        Opcode::True,
        Opcode::JumpIfFalse(6),
        Opcode::Pop,
        Opcode::Const(1),
        Opcode::GetLocal(1),
        Opcode::Print,
        Opcode::Pop,
        Opcode::Jump(1),
        Opcode::Pop,
      ]
    );
  }

  #[test]
  fn assigning_to_a_block_local_uses_set_local() {
    let c = code("if true { let a = 1; a = 2; }");
    assert!(c.contains(&Opcode::SetLocal(1)));
    assert!(!c.contains(&Opcode::SetGlobal(0)));
  }
 
  #[test]
  fn redeclaring_a_local_in_the_same_block_is_an_error() {
    assert!(matches!(
      compile_src("if true { let a = 1; let a = 2; }"),
      Err(CompilerError::AlreadyExistingVariable(_))
    ));
  }

  #[test]
  fn function_definition_and_call() {
    let p = program("fn f(a, b) { return a + b; } print f(1, 2);");
    assert_eq!(
      p.code,
      vec![
        Opcode::Jump(6),
        Opcode::GetLocal(1),
        Opcode::GetLocal(2),
        Opcode::Add,
        Opcode::Ret,
        Opcode::Nil,
        Opcode::Ret,
        Opcode::Const(0),
        Opcode::SetGlobal(0),
        Opcode::GetGlobal(0),
        Opcode::Const(1),
        Opcode::Const(2),
        Opcode::Call(2),
        Opcode::Print,
      ]
    );
    assert_eq!(
      p.constants,
      vec![Value::Function { addr: 1, arity: 2 }, Value::Int(1), Value::Int(2)]
    );
  }

  #[test]
  fn function_without_return_falls_off_with_nil() {
    assert_eq!(
      code("fn f() {}"),
      vec![
        Opcode::Jump(2),
        Opcode::Nil,
        Opcode::Ret,
        Opcode::Const(0),
        Opcode::SetGlobal(0),
      ]
    );
  }
 
  #[test]
  fn bare_return_returns_nil() {
    assert_eq!(
      code("fn f() { return; }"),
      vec![
        Opcode::Jump(4),
        Opcode::Nil,
        Opcode::Ret,
        Opcode::Nil,
        Opcode::Ret,
        Opcode::Const(0),
        Opcode::SetGlobal(0),
      ]
    );
  }

  #[test]
  fn function_reads_globals_with_get_global() {
    let p = program("let g = 1; fn f() { return g; }");
    assert_eq!(
      p.code,
      vec![
        Opcode::Const(0), Opcode::SetGlobal(0),
        Opcode::Jump(4),
        Opcode::GetGlobal(0),
        Opcode::Ret,
        Opcode::Nil,
        Opcode::Ret,
        Opcode::Const(1),
        Opcode::SetGlobal(1),
      ]
    );
    assert_eq!(p.constants[1], Value::Function { addr: 3, arity: 0 });
  }
 
  #[test]
  fn param_shadows_a_global_of_the_same_name() {
    let c = code("let n = 1; fn f(n) { return n; }");
    assert_eq!(c[3], Opcode::GetLocal(1));
  }
 
  #[test]
  fn function_can_call_itself_through_its_global() {
    assert_eq!(
      code("fn f(n) { return f(n); }"),
      vec![
        Opcode::Jump(6),
        Opcode::GetGlobal(0),
        Opcode::GetLocal(1),
        Opcode::Call(1),
        Opcode::Ret,
        Opcode::Nil,
        Opcode::Ret,
        Opcode::Const(0),
        Opcode::SetGlobal(0),
      ]
    );
  }

  #[test]
  fn locals_do_not_leak_out_of_a_function() {
    let c = code("fn f(a) { return a; } print a;");
    assert_eq!(&c[c.len() - 2..], &[Opcode::GetGlobal(1), Opcode::Print]);
  }

  #[test]
  fn return_outside_a_function_is_an_error() {
    assert!(matches!(
      compile_src("return 1;"),
      Err(CompilerError::ReturnOutsideFunction)
    ));
  }
 
  #[test]
  fn nested_functions_are_rejected() {
    assert!(matches!(
      compile_src("fn f() { fn g() {} }"),
      Err(CompilerError::FunctionNotAtTopLevel(_))
    ));
    assert!(matches!(
      compile_src("if true { fn g() {} }"),
      Err(CompilerError::FunctionNotAtTopLevel(_))
    ));
  }
 
  #[test]
  fn duplicate_parameter_names_are_an_error() {
    assert!(matches!(
      compile_src("fn f(a, a) {}"),
      Err(CompilerError::AlreadyExistingVariable(_))
    ));
  }

    #[test]
  fn design_doc_example_program() {
    use Opcode::*;
    let p = program(
      "
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
      ",
    );
 
    assert_eq!(
      p.code,
      vec![
        Const(0), SetGlobal(0),
        GetGlobal(0), Const(1), Add, SetGlobal(0),
        Jump(23),
        GetLocal(1), Const(2), Lt, JumpIfFalse(4),
        Pop, GetLocal(1), Ret, Jump(1),
        Pop,
        GetGlobal(1), GetLocal(1), Const(3), Sub, Call(1),
        GetGlobal(1), GetLocal(1), Const(4), Sub, Call(1),
        Add, Ret,
        Nil, Ret,
        Const(5), SetGlobal(1),
        Const(6), SetGlobal(2),
        GetGlobal(2), Const(7), Lt, JumpIfFalse(10),
        Pop,
        GetGlobal(1), GetGlobal(2), Call(1), Print,
        GetGlobal(2), Const(8), Add, SetGlobal(2),
        Loop(-14),
        Pop,
      ]
    );
    assert_eq!(
      p.constants,
      vec![
        Value::Int(10), Value::Int(1), Value::Int(2), Value::Int(1), Value::Int(2),
        Value::Function { addr: 7, arity: 1 },
        Value::Int(0), Value::Int(10), Value::Int(1),
      ]
    );
    assert_eq!(p.globals, 3);
  }
}
