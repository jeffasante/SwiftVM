use crate::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    LoadConst(Value),
    LoadVar(String),
    StoreVar(String),
    LoadGlobal(String),
    StoreGlobal(String),

    Add,
    Sub,
    Mul,
    Div,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,
    And,
    Or,
    Pop,

    Jump(usize),
    JumpIfFalse(usize),

    CallFunction { name: String, arg_count: usize },
    Return,

    AllocObject { type_name: String },
    GetProp { name: String },
    SetProp { name: String },
    Retain,
    Release,

    CallNative { selector: String, arg_count: usize },
    BuiltinPrint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateField {
    pub name: String,
    pub type_name: String,
    pub default_value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub instructions: Vec<Instruction>,
    pub version: u32,
}

impl Function {
    pub fn new(name: impl Into<String>, params: Vec<&str>, instructions: Vec<Instruction>) -> Self {
        Self {
            name: name.into(),
            params: params.into_iter().map(ToOwned::to_owned).collect(),
            instructions,
            version: 1,
        }
    }

    pub fn with_owned_params(
        name: impl Into<String>,
        params: Vec<String>,
        instructions: Vec<Instruction>,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            instructions,
            version: 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub functions: HashMap<String, Function>,
    pub entry: String,
    pub state_layout: HashMap<String, StateField>,
}

impl Program {
    pub fn with_entry(entry: impl Into<String>) -> Self {
        Self {
            functions: HashMap::new(),
            entry: entry.into(),
            state_layout: HashMap::new(),
        }
    }

    pub fn add_function(&mut self, function: Function) {
        self.functions.insert(function.name.clone(), function);
    }

    pub fn upsert_state_field(&mut self, field: StateField) {
        self.state_layout.insert(field.name.clone(), field);
    }

    pub fn replace_or_add_function(&mut self, mut function: Function) {
        if let Some(existing) = self.functions.get(&function.name) {
            function.version = existing.version.saturating_add(1);
        }
        self.functions.insert(function.name.clone(), function);
    }
}
