use std::fmt;

use crate::{compiler::Program, opcode::{Opcode, Value}};

const MAX_FRAMES: usize = 1024; // Max call depth before stack overflow

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
  StackUnderflow,
  TypeError(&'static str),
  DivisionByZero,
  IntegerOverflow,
  BadConstant(usize),
  BadGlobal(usize),
  UndefinedGlobal(usize),
  BadLocal(usize),
  BadJump(isize),
  NotCallable,
  ArityMismatch { expected: usize, got: usize },
  StackOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
  kind: RuntimeErrorKind,
  ip: usize,
}

impl fmt::Display for RuntimeErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      RuntimeErrorKind::StackUnderflow => write!(f, "stack underflow (internal error)"),
      RuntimeErrorKind::TypeError(msg) => write!(f, "type error: {msg}"),
      RuntimeErrorKind::DivisionByZero => write!(f, "division by zero"),
      RuntimeErrorKind::IntegerOverflow => write!(f, "integer overflow"),
      RuntimeErrorKind::BadConstant(i) => write!(f, "bad constant index {i} (internal error)"),
      RuntimeErrorKind::BadGlobal(i) => write!(f, "bad global index {i} (internal error)"),
      RuntimeErrorKind::BadLocal(i) => write!(f, "bad local slot {i} (internal error)"),
      RuntimeErrorKind::BadJump(off) => write!(f, "bad jump offset {off} (internal error)"),
      RuntimeErrorKind::NotCallable => write!(f, "attempt to call a value that is not a function"),
      RuntimeErrorKind::ArityMismatch { expected, got } => {
        write!(f, "expected {expected} argument(s) but got {got}")
      }
      RuntimeErrorKind::StackOverflow => write!(f, "stack overflow (call depth limit exceeded)"),
      RuntimeErrorKind::UndefinedGlobal(i) => write!(f, "undefined variable (global #{i})"),
    }
  }
}

impl fmt::Display for RuntimeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "runtime error at instruction {}: {}", self.ip, self.kind)
  }
}

pub enum RunResult {
  Success,
  Error(RuntimeError),
}

struct Frame {
  return_ip: usize,
  base: usize,
}

pub struct VM {
  stack: Vec<Value>,
  globals: Vec<Option<Value>>,
  frames: Vec<Frame>,
  ip: usize,
  program: Program,
}

type Res<T> = Result<T, RuntimeErrorKind>;

impl VM {
  pub fn new(program: Program) -> Self {
    Self {
      stack: Vec::new(),
      globals: vec![None; program.globals],
      frames: Vec::new(),
      ip: 0,
      program,
    }
  }

  pub fn run(&mut self) -> RunResult {
    match self.execute() {
      Ok(()) => RunResult::Success,
      Err(kind) => RunResult::Error(RuntimeError {
        kind,
        ip: self.ip.saturating_sub(1),
      }),
    } 
  }

  fn execute(&mut self) -> Res<()> {
    self.stack.push(Value::Nil);
    
    while self.ip < self.program.code.len() {
      let instruction = self.program.code[self.ip];
      self.ip += 1;

      match instruction {
        Opcode::Const(idx) => {
          let val = self
            .program
            .constants
            .get(idx)
            .cloned()
            .ok_or(RuntimeErrorKind::BadConstant(idx))?;
          self.stack.push(val);
        }
        Opcode::True => self.stack.push(Value::Bool(true)),
        Opcode::False => self.stack.push(Value::Bool(false)),
        Opcode::Nil => self.stack.push(Value::Nil),
        Opcode::Pop => {
          self.pop()?;
        }
        Opcode::GetLocal(i) => {
          let slot = self.base() + i;
          let val = self
            .stack
            .get(slot)
            .cloned()
            .ok_or(RuntimeErrorKind::BadLocal(i))?;
          
          self.stack.push(val);
        }
        Opcode::SetLocal(i) => {
          let val = self.pop()?;
          let slot = self.base() + i;

          *self
            .stack
            .get_mut(slot)
            .ok_or(RuntimeErrorKind::BadLocal(i))? = val;
        }
        Opcode::GetGlobal(i) => {
          let val = self
            .globals
            .get(i)
            .ok_or(RuntimeErrorKind::BadGlobal(i))?
            .clone()
            .ok_or(RuntimeErrorKind::UndefinedGlobal(i))?;

          self.stack.push(val);
        }
        Opcode::SetGlobal(i) => {
          let val = self.pop()?;

          *self
            .globals
            .get_mut(i)
            .ok_or(RuntimeErrorKind::BadGlobal(i))? = Some(val);
        }
        Opcode::Add => {
          let (a, b) = self.pop_ints()?;
          let r = a.checked_add(b).ok_or(RuntimeErrorKind::IntegerOverflow)?;
          self.stack.push(Value::Int(r));
        }
        Opcode::Sub => {
          let (a, b) = self.pop_ints()?;
          let r = a.checked_sub(b).ok_or(RuntimeErrorKind::IntegerOverflow)?;
          self.stack.push(Value::Int(r));
        }
        Opcode::Mul => {
          let (a, b) = self.pop_ints()?;
          let r = a.checked_mul(b).ok_or(RuntimeErrorKind::IntegerOverflow)?;
          self.stack.push(Value::Int(r));
        }
        Opcode::Div => {
          let (a, b) = self.pop_ints()?;
          if b == 0 {
            return Err(RuntimeErrorKind::DivisionByZero);
          }
          let r = a.checked_div(b).ok_or(RuntimeErrorKind::IntegerOverflow)?;
          self.stack.push(Value::Int(r));
        }
        Opcode::Mod => {
          let (a, b) = self.pop_ints()?;
          if b == 0 {
            return Err(RuntimeErrorKind::DivisionByZero);
          }
          let r = a.checked_rem(b).ok_or(RuntimeErrorKind::IntegerOverflow)?;
          self.stack.push(Value::Int(r));
        }
        Opcode::Neg => match self.pop()? {
          Value::Int(n) => {
            let r = n.checked_neg().ok_or(RuntimeErrorKind::IntegerOverflow)?;
            self.stack.push(Value::Int(r));
          }
          _ => return Err(RuntimeErrorKind::TypeError("operand of '-' must be an integer")),
        },
        Opcode::Not => {
          let v = self.pop()?;
          self.stack.push(Value::Bool(!Self::is_truthy(&v)));
        }
        Opcode::Eq => {
          let b = self.pop()?;
          let a = self.pop()?;
          self.stack.push(Value::Bool(a == b));
        }
        Opcode::Lt => {
          let (a, b) = self.pop_ints()?;
          self.stack.push(Value::Bool(a < b));
        }
        Opcode::Le => {
          let (a, b) = self.pop_ints()?;
          self.stack.push(Value::Bool(a <= b));
        }
        Opcode::Jump(offset) => self.jump(offset)?,
        Opcode::JumpIfFalse(offset) => {
          let cond = self.stack.last().ok_or(RuntimeErrorKind::StackUnderflow)?;
          if !Self::is_truthy(&cond) {
            self.jump(offset)?;
          }
        }
        Opcode::Loop(offset) => {
          self.jump(-offset.checked_abs().ok_or(RuntimeErrorKind::BadJump(offset))?)?;
        }
        Opcode::Call(argc) => {
          if self.frames.len() >= MAX_FRAMES {
            return Err(RuntimeErrorKind::StackOverflow);
          }

          let callee_idx = self
            .stack
            .len()
            .checked_sub(argc + 1)
            .ok_or(RuntimeErrorKind::StackUnderflow)?;

          match &self.stack[callee_idx] {
            Value::Function { addr, arity } => {
              if *arity != argc {
                return Err(RuntimeErrorKind::ArityMismatch { expected: *arity, got: argc, });
              }

              self.frames.push(Frame {
                return_ip: self.ip,
                base: callee_idx,
              });
              
              self.ip = *addr;
            }
            _ => return Err(RuntimeErrorKind::NotCallable),
          }
        }
        Opcode::Ret => {
          let result = self.pop()?;
          match self.frames.pop() {
            None => return Ok(()),
            Some(frame) => {
              self.stack.truncate(frame.base);
              self.stack.push(result);
              self.ip = frame.return_ip;
            }
          }
        }
        Opcode::Print => {
          let v = self.pop()?;
          println!("{}", Self::format_value(&v));
        }

      }
    }
    
    Ok(())
  }

  fn pop(&mut self) -> Res<Value> {
    self.stack.pop().ok_or(RuntimeErrorKind::StackUnderflow)
  }

  fn pop_ints(&mut self) -> Res<(i64, i64)> {
    let b = self.pop()?;
    let a = self.pop()?;

    match (a, b) {
      (Value::Int(a), Value::Int(b)) => Ok((a, b)),
      _ => Err(RuntimeErrorKind::TypeError("operands must be integers")),
    }
  }

  fn base(&self) -> usize {
    self.frames.last().map_or(0, |f| f.base)
  }

  fn is_truthy(v: &Value) -> bool {
    !matches!(v, Value::Bool(false) | Value::Nil)
  }

  fn jump(&mut self, offset: isize) -> Res<()> {
    let target = (self.ip as isize)
      .checked_add(offset)
      .filter(|&t| t >= 0 && t as usize <= self.program.code.len())
      .ok_or(RuntimeErrorKind::BadJump(offset))?;

    self.ip = target as usize;
    Ok(())
  }
  
  fn format_value(v: &Value) -> String {
    match v {
      Value::Int(n) => n.to_string(),
      Value::Bool(b) => b.to_string(),
      Value::Nil => "nil".to_string(),
      Value::Function { addr, arity } => format!("<fn @{} /{}>", addr, arity),
    }
  }
}
