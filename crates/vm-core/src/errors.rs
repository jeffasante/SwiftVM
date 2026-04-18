use thiserror::Error;

#[derive(Debug, Error)]
pub enum VMError {
    #[error("function not found: {0}")]
    FunctionNotFound(String),

    #[error("instruction pointer out of range in function `{function}` at pc {pc}")]
    InstructionOutOfRange { function: String, pc: usize },

    #[error("stack underflow during {op}")]
    StackUnderflow { op: &'static str },

    #[error("undefined variable `{name}`")]
    UndefinedVariable { name: String },

    #[error("type error: {0}")]
    TypeError(String),

    #[error("invalid jump target {target}")]
    InvalidJumpTarget { target: usize },

    #[error("expected {expected} arguments for function `{name}`, got {got}")]
    ArityMismatch {
        name: String,
        expected: usize,
        got: usize,
    },

    #[error("object not found in heap: {id}")]
    ObjectNotFound { id: u64 },

    #[error("native selector `{selector}` not registered")]
    NativeSelectorNotFound { selector: String },
}
