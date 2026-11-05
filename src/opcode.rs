#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
  Int(i64),
  Bool(bool),
  Nil,
  Function {
    addr: usize,
    arity: usize
  }, 
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opcode {
  // Constants & Literals
  Const(usize),
  True,
  False,
  Nil,
  
  // Stack
  Pop,
  
  // Variables
  GetLocal(usize),
  SetLocal(usize),
  GetGlobal(usize),
  SetGlobal(usize),
  
  // Arithmetic
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Neg,
  
  // Logic / Compare
  Not,
  Eq,
  Lt,
  Le,
  // The rest of the logical operators are derived
  
  // Control Flow
  Jump(isize),
  JumpIfFalse(isize),
  Loop(isize), // Jump backward
  
  // Functions & I/O
  Call(usize), // argc
  Ret,
  Print,
}
