pub mod bytecode;
pub mod errors;
pub mod heap;
pub mod instructions;
pub mod value;
pub mod vm;

pub use bytecode::{decode_program, encode_program, BytecodeError};
pub use errors::VMError;
pub use heap::{ArcHeap, SwiftObject};
pub use instructions::{Function, Instruction, Program, StateField};
pub use value::Value;
pub use vm::VM;
