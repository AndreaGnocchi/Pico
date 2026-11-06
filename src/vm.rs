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

#[cfg(test)]
mod tests {
  use super::*;
  use Opcode;

  fn run_vm(code: Vec<Opcode>, constants: Vec<Value>, globals: usize) -> (RunResult, VM) {
    let mut vm = VM::new(Program {
      code,
      constants,
      globals,
    });
    let result = vm.run();
    (result, vm)
  }

  fn ok(code: Vec<Opcode>, constants: Vec<Value>, globals: usize) -> VM {
    match run_vm(code, constants, globals) {
      (RunResult::Success, vm) => vm,
      (RunResult::Error(e), _) => panic!("unexpected runtime error: {e}"),
    }
  }

  fn err(code: Vec<Opcode>, constants: Vec<Value>, globals: usize) -> RuntimeError {
    match run_vm(code, constants, globals) {
      (RunResult::Error(e), _) => e,
      (RunResult::Success, _) => panic!("expected a runtime error"),
    }
  }

  fn top(vm: &VM) -> Value {
    vm.stack.last().cloned().expect("stack is empty")
  }

  fn bin(a: i64, op: Opcode, b: i64) -> Value {
    let vm = ok(
      vec![Opcode::Const(0), Opcode::Const(1), op],
      vec![Value::Int(a), Value::Int(b)],
      0,
    );
    top(&vm)
  }

  fn bin_err(a: i64, op: Opcode, b: i64) -> RuntimeErrorKind {
    err(
      vec![Opcode::Const(0), Opcode::Const(1), op],
      vec![Value::Int(a), Value::Int(b)],
      0,
    )
    .kind
  }

  fn int(n: i64) -> Value {
    Value::Int(n)
  }
  
  #[test]
  fn empty_program_succeeds_with_initial_nil() {
    let vm = ok(vec![], vec![], 0);
    assert_eq!(vm.stack, vec![Value::Nil]);
  }

  #[test]
  fn literals_push_values() {
    let vm = ok(vec![Opcode::True, Opcode::False, Opcode::Nil], vec![], 0);
    assert_eq!(
      vm.stack,
      vec![Value::Nil, Value::Bool(true), Value::Bool(false), Value::Nil]
    );
  }

  #[test]
  fn const_pushes_from_constant_pool() {
    let vm = ok(vec![Opcode::Const(1)], vec![int(1), int(2)], 0);
    assert_eq!(top(&vm), int(2));
  }

  #[test]
  fn bad_constant_index() {
    let e = err(vec![Opcode::Const(5)], vec![int(1)], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadConstant(5));
  }

  #[test]
  fn pop_removes_top() {
    let vm = ok(vec![Opcode::True, Opcode::Pop], vec![], 0);
    assert_eq!(vm.stack, vec![Value::Nil]);
  }

  #[test]
  fn pop_on_empty_stack_underflows() {
    let e = err(vec![Opcode::Pop, Opcode::Pop], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::StackUnderflow);
  }

  #[test]
  fn add_sub_mul() {
    assert_eq!(bin(2, Opcode::Add, 3), int(5));
    assert_eq!(bin(2, Opcode::Sub, 3), int(-1));
    assert_eq!(bin(4, Opcode::Mul, -3), int(-12));
  }

  #[test]
  fn sub_operand_order() {
    assert_eq!(bin(10, Opcode::Sub, 3), int(7));
  }

  #[test]
  fn div_truncates_toward_zero() {
    assert_eq!(bin(7, Opcode::Div, 2), int(3));
    assert_eq!(bin(-7, Opcode::Div, 2), int(-3));
    assert_eq!(bin(7, Opcode::Div, -2), int(-3));
  }

  #[test]
  fn mod_takes_sign_of_dividend() {
    assert_eq!(bin(7, Opcode::Mod, 3), int(1));
    assert_eq!(bin(-7, Opcode::Mod, 3), int(-1));
    assert_eq!(bin(7, Opcode::Mod, -3), int(1));
  }

  #[test]
  fn neg() {
    let vm = ok(vec![Opcode::Const(0), Opcode::Neg], vec![int(5)], 0);
    assert_eq!(top(&vm), int(-5));
  }

  #[test]
  fn division_by_zero() {
    assert_eq!(bin_err(1, Opcode::Div, 0), RuntimeErrorKind::DivisionByZero);
    assert_eq!(bin_err(1, Opcode::Mod, 0), RuntimeErrorKind::DivisionByZero);
  }

  #[test]
  fn add_overflow() {
    assert_eq!(bin_err(i64::MAX, Opcode::Add, 1), RuntimeErrorKind::IntegerOverflow);
  }

  #[test]
  fn sub_overflow() {
    assert_eq!(bin_err(i64::MIN, Opcode::Sub, 1), RuntimeErrorKind::IntegerOverflow);
  }

  #[test]
  fn mul_overflow() {
    assert_eq!(bin_err(i64::MAX, Opcode::Mul, 2), RuntimeErrorKind::IntegerOverflow);
  }

  #[test]
  fn div_min_by_minus_one_overflows() {
    assert_eq!(bin_err(i64::MIN, Opcode::Div, -1), RuntimeErrorKind::IntegerOverflow);
  }

  #[test]
  fn mod_min_by_minus_one_overflows() {
    assert_eq!(bin_err(i64::MIN, Opcode::Mod, -1), RuntimeErrorKind::IntegerOverflow);
  }

  #[test]
  fn neg_min_overflows() {
    let e = err(vec![Opcode::Const(0), Opcode::Neg], vec![int(i64::MIN)], 0);
    assert_eq!(e.kind, RuntimeErrorKind::IntegerOverflow);
  }

  #[test]
  fn arithmetic_extremes_that_do_not_overflow() {
    assert_eq!(bin(i64::MAX, Opcode::Add, 0), int(i64::MAX));
    assert_eq!(bin(i64::MIN, Opcode::Add, i64::MAX), int(-1));
    assert_eq!(bin(i64::MIN, Opcode::Div, 1), int(i64::MIN));
  }

  #[test]
  fn arithmetic_type_errors() {
    for op in [Opcode::Add, Opcode::Sub, Opcode::Mul, Opcode::Div, Opcode::Mod, Opcode::Lt, Opcode::Le] {
      let e = err(vec![Opcode::True, Opcode::Const(0), op], vec![int(1)], 0);
      assert_eq!(
        e.kind,
        RuntimeErrorKind::TypeError("operands must be integers"),
        "op = {op:?}"
      );
    }
  }

  #[test]
  fn neg_type_error() {
    let e = err(vec![Opcode::True, Opcode::Neg], vec![], 0);
    assert_eq!(
      e.kind,
      RuntimeErrorKind::TypeError("operand of '-' must be an integer")
    );
  }

  #[test]
  fn binary_op_with_too_few_operands_underflows() {
    let e = err(vec![Opcode::Pop, Opcode::Const(0), Opcode::Add], vec![int(1)], 0);
    assert_eq!(e.kind, RuntimeErrorKind::StackUnderflow);
  }

  #[test]
  fn not_uses_truthiness() {
    let cases = [
      (vec![Opcode::True], false),
      (vec![Opcode::False], true),
      (vec![Opcode::Nil], true),
      (vec![Opcode::Const(0)], false),
    ];
    for (mut code, expected) in cases {
      code.push(Opcode::Not);
      let vm = ok(code, vec![int(0)], 0);
      assert_eq!(top(&vm), Value::Bool(expected));
    }
  }

  #[test]
  fn eq_same_type() {
    let vm = ok(vec![Opcode::Const(0), Opcode::Const(0), Opcode::Eq], vec![int(3)], 0);
    assert_eq!(top(&vm), Value::Bool(true));
    let vm = ok(vec![Opcode::Const(0), Opcode::Const(1), Opcode::Eq], vec![int(3), int(4)], 0);
    assert_eq!(top(&vm), Value::Bool(false));
  }

  #[test]
  fn eq_nil_and_bool() {
    let vm = ok(vec![Opcode::Nil, Opcode::Nil, Opcode::Eq], vec![], 0);
    assert_eq!(top(&vm), Value::Bool(true));
    let vm = ok(vec![Opcode::True, Opcode::True, Opcode::Eq], vec![], 0);
    assert_eq!(top(&vm), Value::Bool(true));
    let vm = ok(vec![Opcode::True, Opcode::False, Opcode::Eq], vec![], 0);
    assert_eq!(top(&vm), Value::Bool(false));
  }

  #[test]
  fn eq_different_types_is_false() {
    let vm = ok(vec![Opcode::Const(0), Opcode::Nil, Opcode::Eq], vec![int(0)], 0);
    assert_eq!(top(&vm), Value::Bool(false));
    let vm = ok(vec![Opcode::Const(0), Opcode::False, Opcode::Eq], vec![int(0)], 0);
    assert_eq!(top(&vm), Value::Bool(false));
  }

  #[test]
  fn eq_functions() {
    let f = Value::Function { addr: 1, arity: 0 };
    let g = Value::Function { addr: 2, arity: 0 };
    let vm = ok(vec![Opcode::Const(0), Opcode::Const(0), Opcode::Eq], vec![f.clone()], 0);
    assert_eq!(top(&vm), Value::Bool(true));
    let vm = ok(vec![Opcode::Const(0), Opcode::Const(1), Opcode::Eq], vec![f, g], 0);
    assert_eq!(top(&vm), Value::Bool(false));
  }

  #[test]
  fn lt_and_le() {
    assert_eq!(bin(1, Opcode::Lt, 2), Value::Bool(true));
    assert_eq!(bin(2, Opcode::Lt, 2), Value::Bool(false));
    assert_eq!(bin(3, Opcode::Lt, 2), Value::Bool(false));
    assert_eq!(bin(1, Opcode::Le, 2), Value::Bool(true));
    assert_eq!(bin(2, Opcode::Le, 2), Value::Bool(true));
    assert_eq!(bin(3, Opcode::Le, 2), Value::Bool(false));
    assert_eq!(bin(-5, Opcode::Lt, 0), Value::Bool(true));
  }

  #[test]
  fn truthiness() {
    assert!(!VM::is_truthy(&Value::Nil));
    assert!(!VM::is_truthy(&Value::Bool(false)));
    assert!(VM::is_truthy(&Value::Bool(true)));
    assert!(VM::is_truthy(&int(0)));
    assert!(VM::is_truthy(&Value::Function { addr: 0, arity: 0 }));
  }

  #[test]
  fn set_then_get_global() {
    let vm = ok(
      vec![Opcode::Const(0), Opcode::SetGlobal(0), Opcode::GetGlobal(0)],
      vec![int(42)],
      1,
    );
    assert_eq!(top(&vm), int(42));
    assert_eq!(vm.globals[0], Some(int(42)));
  }

  #[test]
  fn set_global_pops_value() {
    let vm = ok(vec![Opcode::True, Opcode::SetGlobal(0)], vec![], 1);
    assert_eq!(vm.stack, vec![Value::Nil]);
  }

  #[test]
  fn global_can_be_overwritten() {
    let vm = ok(
      vec![
        Opcode::Const(0),
        Opcode::SetGlobal(0),
        Opcode::Const(1),
        Opcode::SetGlobal(0),
      ],
      vec![int(1), int(2)],
      1,
    );
    assert_eq!(vm.globals[0], Some(int(2)));
  }

  #[test]
  fn globals_start_undefined() {
    let vm = ok(vec![], vec![], 3);
    assert_eq!(vm.globals, vec![None, None, None]);
  }

  #[test]
  fn get_undefined_global() {
    let e = err(vec![Opcode::GetGlobal(0)], vec![], 1);
    assert_eq!(e.kind, RuntimeErrorKind::UndefinedGlobal(0));
  }

  #[test]
  fn get_global_out_of_range() {
    let e = err(vec![Opcode::GetGlobal(0)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadGlobal(0));
  }

  #[test]
  fn set_global_out_of_range() {
    let e = err(vec![Opcode::True, Opcode::SetGlobal(2)], vec![], 1);
    assert_eq!(e.kind, RuntimeErrorKind::BadGlobal(2));
  }

  #[test]
  fn get_local_copies_value() {
    let vm = ok(vec![Opcode::GetLocal(0)], vec![], 0);
    assert_eq!(vm.stack, vec![Value::Nil, Value::Nil]);
  }

  #[test]
  fn set_local_overwrites_slot() {
    let vm = ok(vec![Opcode::Const(0), Opcode::SetLocal(0)], vec![int(5)], 0);
    assert_eq!(vm.stack, vec![int(5)]);
  }

  #[test]
  fn set_then_get_local() {
    let vm = ok(
      vec![Opcode::Const(0), Opcode::SetLocal(0), Opcode::GetLocal(0)],
      vec![int(5)],
      0,
    );
    assert_eq!(vm.stack, vec![int(5), int(5)]);
  }

  #[test]
  fn get_local_out_of_range() {
    let e = err(vec![Opcode::GetLocal(99)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadLocal(99));
  }

  #[test]
  fn set_local_out_of_range() {
    let e = err(vec![Opcode::True, Opcode::SetLocal(99)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadLocal(99));
  }

  #[test]
  fn jump_forward_skips_instructions() {
    let vm = ok(vec![Opcode::Jump(1), Opcode::True, Opcode::False], vec![], 0);
    assert_eq!(vm.stack, vec![Value::Nil, Value::Bool(false)]);
  }

  #[test]
  fn jump_zero_is_a_noop() {
    let vm = ok(vec![Opcode::Jump(0), Opcode::True], vec![], 0);
    assert_eq!(top(&vm), Value::Bool(true));
  }

  #[test]
  fn jump_to_end_of_code_is_allowed() {
    let vm = ok(vec![Opcode::Jump(1), Opcode::True], vec![], 0);
    assert_eq!(vm.stack, vec![Value::Nil]);
  }

  #[test]
  fn jump_past_end_is_bad() {
    let e = err(vec![Opcode::Jump(100)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadJump(100));
  }

  #[test]
  fn jump_before_start_is_bad() {
    let e = err(vec![Opcode::Jump(-100)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadJump(-100));
  }

  #[test]
  fn jump_offset_overflow_is_bad() {
    let e = err(vec![Opcode::Jump(isize::MAX)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadJump(isize::MAX));
  }

  #[test]
  fn jump_if_false_jumps_on_false_and_nil_and_keeps_condition() {
    for cond in [Opcode::False, Opcode::Nil] {
      let vm = ok(vec![cond, Opcode::JumpIfFalse(1), Opcode::True], vec![], 0);
      assert_eq!(vm.stack.len(), 2, "cond = {cond:?}");
      assert_ne!(top(&vm), Value::Bool(true), "cond = {cond:?}");
    }
  }

  #[test]
  fn jump_if_false_falls_through_on_truthy() {
    let vm = ok(vec![Opcode::True, Opcode::JumpIfFalse(1), Opcode::False], vec![], 0);
    assert_eq!(
      vm.stack,
      vec![Value::Nil, Value::Bool(true), Value::Bool(false)]
    );

    let vm = ok(vec![Opcode::Const(0), Opcode::JumpIfFalse(1), Opcode::False], vec![int(0)], 0);
    assert_eq!(vm.stack.len(), 3);
  }

  #[test]
  fn jump_if_false_only_validates_offset_when_taken() {
    let vm = ok(vec![Opcode::True, Opcode::JumpIfFalse(100)], vec![], 0);
    assert_eq!(vm.stack.len(), 2);

    let e = err(vec![Opcode::False, Opcode::JumpIfFalse(100)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadJump(100));
  }

  #[test]
  fn jump_if_false_on_empty_stack_underflows() {
    let e = err(vec![Opcode::Pop, Opcode::JumpIfFalse(0)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::StackUnderflow);
  }

  #[test]
  fn loop_counts_to_three() {
    let code = vec![
      Opcode::Const(0),
      Opcode::SetGlobal(0),
      Opcode::GetGlobal(0),
      Opcode::Const(1),
      Opcode::Lt,
      Opcode::JumpIfFalse(6),
      Opcode::Pop,
      Opcode::GetGlobal(0),
      Opcode::Const(2),
      Opcode::Add,
      Opcode::SetGlobal(0),
      Opcode::Loop(10),
    ];
    let vm = ok(code, vec![int(0), int(3), int(1)], 1);
    assert_eq!(vm.globals[0], Some(int(3)));
    assert_eq!(vm.stack, vec![Value::Nil, Value::Bool(false)]);
  }

  #[test]
  fn loop_before_start_is_bad() {
    let e = err(vec![Opcode::Loop(100)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadJump(-100));
  }

  #[test]
  fn loop_with_min_offset_is_bad() {
    let e = err(vec![Opcode::Loop(isize::MIN)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::BadJump(isize::MIN));
  }

  #[test]
  fn call_and_return_with_one_argument() {
    let code = vec![
      Opcode::Const(0),
      Opcode::Const(1),
      Opcode::Call(1),
      Opcode::SetGlobal(0),
      Opcode::Nil,
      Opcode::Ret,
      Opcode::GetLocal(1),
      Opcode::Const(2),
      Opcode::Add,
      Opcode::Ret,
    ];
    let consts = vec![Value::Function { addr: 6, arity: 1 }, int(20), int(1)];
    let vm = ok(code, consts, 1);
    assert_eq!(vm.globals[0], Some(int(21)));
    assert_eq!(vm.stack, vec![Value::Nil]);
    assert!(vm.frames.is_empty());
  }

  #[test]
  fn call_with_multiple_arguments_preserves_order() {
    let code = vec![
      Opcode::Const(0),
      Opcode::Const(1),
      Opcode::Const(2),
      Opcode::Call(2),
      Opcode::SetGlobal(0),
      Opcode::Nil,
      Opcode::Ret,
      Opcode::GetLocal(1),
      Opcode::GetLocal(2),
      Opcode::Sub,
      Opcode::Ret,
    ];
    let consts = vec![Value::Function { addr: 7, arity: 2 }, int(10), int(3)];
    let vm = ok(code, consts, 1);
    assert_eq!(vm.globals[0], Some(int(7)));
  }

  #[test]
  fn call_with_no_arguments() {
    let code = vec![
      Opcode::Const(0),
      Opcode::Call(0),
      Opcode::SetGlobal(0),
      Opcode::Nil,
      Opcode::Ret,
      Opcode::Const(1),
      Opcode::Ret,
    ];
    let consts = vec![Value::Function { addr: 5, arity: 0 }, int(7)];
    let vm = ok(code, consts, 1);
    assert_eq!(vm.globals[0], Some(int(7)));
  }

  #[test]
  fn call_leaves_caller_stack_intact() {
    let code = vec![
      Opcode::Const(1),
      Opcode::Const(0),
      Opcode::Call(0),
      Opcode::Nil,
      Opcode::Ret,
      Opcode::Const(2),
      Opcode::Ret,
    ];
    let consts = vec![Value::Function { addr: 5, arity: 0 }, int(99), int(7)];
    let vm = ok(code, consts, 0);
    assert_eq!(vm.stack, vec![Value::Nil, int(99), int(7)]);
  }

  #[test]
  fn function_can_assign_to_its_parameter() {
    let code = vec![
      Opcode::Const(0),
      Opcode::Const(1),
      Opcode::Call(1),
      Opcode::SetGlobal(0),
      Opcode::Nil,
      Opcode::Ret,
      Opcode::Const(2),
      Opcode::SetLocal(1),
      Opcode::GetLocal(1),
      Opcode::Ret,
    ];
    let consts = vec![Value::Function { addr: 6, arity: 1 }, int(1), int(5)];
    let vm = ok(code, consts, 1);
    assert_eq!(vm.globals[0], Some(int(5)));
  }

  #[test]
  fn recursion_sum_to_n() {
    let code = vec![
      Opcode::Const(0),
      Opcode::Const(1),
      Opcode::Call(1),
      Opcode::SetGlobal(0),
      Opcode::Nil,
      Opcode::Ret,
      Opcode::GetLocal(1),
      Opcode::Const(2),
      Opcode::Eq,
      Opcode::JumpIfFalse(3),
      Opcode::Pop,
      Opcode::Const(2),
      Opcode::Ret,
      Opcode::Pop,
      Opcode::GetLocal(1),
      Opcode::GetLocal(0),
      Opcode::GetLocal(1),
      Opcode::Const(3),
      Opcode::Sub,
      Opcode::Call(1),
      Opcode::Add,
      Opcode::Ret,
    ];
    let consts = vec![
      Value::Function { addr: 6, arity: 1 },
      int(4),
      int(0),
      int(1),
    ];
    let vm = ok(code, consts, 1);
    assert_eq!(vm.globals[0], Some(int(10)));
    assert_eq!(vm.stack, vec![Value::Nil]);
    assert!(vm.frames.is_empty());
  }

  #[test]
  fn arity_mismatch_too_few() {
    let code = vec![Opcode::Const(0), Opcode::Call(0)];
    let e = err(code, vec![Value::Function { addr: 0, arity: 1 }], 0);
    assert_eq!(
      e.kind,
      RuntimeErrorKind::ArityMismatch { expected: 1, got: 0 }
    );
  }

  #[test]
  fn arity_mismatch_too_many() {
    let code = vec![Opcode::Const(0), Opcode::Const(1), Opcode::Call(1)];
    let e = err(code, vec![Value::Function { addr: 0, arity: 0 }, int(1)], 0);
    assert_eq!(
      e.kind,
      RuntimeErrorKind::ArityMismatch { expected: 0, got: 1 }
    );
  }

  #[test]
  fn calling_a_non_function() {
    let e = err(vec![Opcode::Const(0), Opcode::Call(0)], vec![int(1)], 0);
    assert_eq!(e.kind, RuntimeErrorKind::NotCallable);
  }

  #[test]
  fn calling_nil() {
    let e = err(vec![Opcode::Nil, Opcode::Call(0)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::NotCallable);
  }

  #[test]
  fn call_with_not_enough_stack_underflows() {
    let e = err(vec![Opcode::Pop, Opcode::Call(0)], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::StackUnderflow);
  }

  #[test]
  fn infinite_recursion_overflows_the_call_stack() {
    let code = vec![
      Opcode::Const(0),
      Opcode::Call(0),
      Opcode::Ret,
      Opcode::GetLocal(0),
      Opcode::Call(0),
      Opcode::Ret,
    ];
    let e = err(code, vec![Value::Function { addr: 3, arity: 0 }], 0);
    assert_eq!(e.kind, RuntimeErrorKind::StackOverflow);
  }

  #[test]
  fn recursion_just_under_the_limit_succeeds() {
    let depth = (MAX_FRAMES - 1) as i64;
    let code = vec![
      Opcode::Const(0),
      Opcode::Const(1),
      Opcode::Call(1),
      Opcode::SetGlobal(0),
      Opcode::Nil,
      Opcode::Ret,
      Opcode::GetLocal(1),
      Opcode::Const(2),
      Opcode::Eq,
      Opcode::JumpIfFalse(3),
      Opcode::Pop,
      Opcode::Const(2),
      Opcode::Ret,
      Opcode::Pop,
      Opcode::GetLocal(0),
      Opcode::GetLocal(1),
      Opcode::Const(3),
      Opcode::Sub,
      Opcode::Call(1),
      Opcode::Ret,
    ];
    let consts = vec![
      Value::Function { addr: 6, arity: 1 },
      int(depth),
      int(0),
      int(1),
    ];
    let vm = ok(code, consts, 1);
    assert_eq!(vm.globals[0], Some(int(0)));
  }

  #[test]
  fn ret_at_top_level_ends_the_program() {
    let vm = ok(vec![Opcode::Nil, Opcode::Ret, Opcode::SetGlobal(0)], vec![], 1);
    assert_eq!(vm.globals[0], None);
  }

  #[test]
  fn ret_with_empty_stack_underflows() {
    let e = err(vec![Opcode::Pop, Opcode::Ret], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::StackUnderflow);
  }

  #[test]
  fn print_pops_its_operand() {
    let vm = ok(vec![Opcode::Const(0), Opcode::Print], vec![int(1)], 0);
    assert_eq!(vm.stack, vec![Value::Nil]);
  }

  #[test]
  fn print_on_empty_stack_underflows() {
    let e = err(vec![Opcode::Pop, Opcode::Print], vec![], 0);
    assert_eq!(e.kind, RuntimeErrorKind::StackUnderflow);
  }

  #[test]
  fn format_value_output() {
    assert_eq!(VM::format_value(&int(-12)), "-12");
    assert_eq!(VM::format_value(&Value::Bool(true)), "true");
    assert_eq!(VM::format_value(&Value::Bool(false)), "false");
    assert_eq!(VM::format_value(&Value::Nil), "nil");
    assert_eq!(
      VM::format_value(&Value::Function { addr: 3, arity: 2 }),
      "<fn @3 /2>"
    );
  }

  #[test]
  fn error_ip_points_at_the_failing_instruction() {
    let e = err(vec![Opcode::True, Opcode::True, Opcode::Add], vec![], 0);
    assert_eq!(e.ip, 2);
  }

  #[test]
  fn error_ip_for_first_instruction() {
    let e = err(vec![Opcode::Const(9)], vec![], 0);
    assert_eq!(e.ip, 0);
  }

  #[test]
  fn runtime_error_display() {
    let e = err(vec![Opcode::True, Opcode::True, Opcode::Add], vec![], 0);
    assert_eq!(
      e.to_string(),
      "runtime error at instruction 2: type error: operands must be integers"
    );
  }

  #[test]
  fn error_kind_display_messages() {
    use RuntimeErrorKind::*;
    assert_eq!(DivisionByZero.to_string(), "division by zero");
    assert_eq!(IntegerOverflow.to_string(), "integer overflow");
    assert_eq!(
      ArityMismatch { expected: 2, got: 1 }.to_string(),
      "expected 2 argument(s) but got 1"
    );
    assert_eq!(
      NotCallable.to_string(),
      "attempt to call a value that is not a function"
    );
    assert_eq!(
      UndefinedGlobal(4).to_string(),
      "undefined variable (global #4)"
    );
    assert_eq!(
      StackOverflow.to_string(),
      "stack overflow (call depth limit exceeded)"
    );
  }
}
