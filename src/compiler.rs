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
