use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Object(u64),
    Nil,
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(v) => *v,
            Value::Nil => false,
            Value::Int(v) => *v != 0,
            Value::Str(v) => !v.is_empty(),
            Value::Object(_) => true,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_object_id(&self) -> Option<u64> {
        if let Value::Object(id) = self {
            Some(*id)
        } else {
            None
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Str(v) => write!(f, "{v}"),
            Value::Object(id) => write!(f, "<Object#{id}>"),
            Value::Nil => write!(f, "nil"),
        }
    }
}
