use crate::{Function, Instruction, Program, StateField, Value};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use thiserror::Error;

const MAGIC: &[u8; 4] = b"SWBC";
const VERSION: u16 = 3;

#[derive(Debug, Error)]
pub enum BytecodeError {
    #[error("invalid magic header")]
    InvalidMagic,

    #[error("unsupported bytecode version: {0}")]
    UnsupportedVersion(u16),

    #[error("malformed bytecode: {0}")]
    Malformed(String),
}

pub fn encode_program(program: &Program) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    write_u16(&mut out, VERSION);
    write_u16(&mut out, 0); // flags

    write_string(&mut out, &program.entry);

    let mut state_names = program.state_layout.keys().cloned().collect::<Vec<_>>();
    state_names.sort();
    write_u32(&mut out, state_names.len() as u32);
    for name in state_names {
        let field = &program.state_layout[&name];
        write_string(&mut out, &field.name);
        write_string(&mut out, &field.type_name);
        match &field.default_value {
            Some(v) => {
                out.push(1);
                encode_value(&mut out, v);
            }
            None => out.push(0),
        }
    }

    write_u32(&mut out, program.functions.len() as u32);

    let mut names = program.functions.keys().cloned().collect::<Vec<_>>();
    names.sort();

    for name in names {
        let function = &program.functions[&name];
        write_string(&mut out, &function.name);
        write_u16(&mut out, function.params.len() as u16);
        for param in &function.params {
            write_string(&mut out, param);
        }
        write_u32(&mut out, function.version);
        write_u32(&mut out, function.instructions.len() as u32);
        for instr in &function.instructions {
            encode_instruction(&mut out, instr);
        }
    }

    out
}

pub fn decode_program(bytes: &[u8]) -> Result<Program, BytecodeError> {
    let mut c = Cursor::new(bytes);

    let mut magic = [0u8; 4];
    c.read_exact(&mut magic)
        .map_err(|_| BytecodeError::Malformed("missing header".to_string()))?;
    if &magic != MAGIC {
        return Err(BytecodeError::InvalidMagic);
    }

    let version = read_u16(&mut c)?;
    if version != VERSION {
        return Err(BytecodeError::UnsupportedVersion(version));
    }

    let _flags = read_u16(&mut c)?;
    let entry = read_string(&mut c)?;

    let state_count = read_u32(&mut c)? as usize;
    let mut state_layout = HashMap::new();
    for _ in 0..state_count {
        let name = read_string(&mut c)?;
        let type_name = read_string(&mut c)?;
        let has_default = read_u8(&mut c)? != 0;
        let default_value = if has_default { Some(decode_value(&mut c)?) } else { None };
        state_layout.insert(
            name.clone(),
            StateField {
                name,
                type_name,
                default_value,
            },
        );
    }

    let function_count = read_u32(&mut c)? as usize;

    let mut functions = HashMap::new();
    for _ in 0..function_count {
        let name = read_string(&mut c)?;
        let param_count = read_u16(&mut c)? as usize;
        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            params.push(read_string(&mut c)?);
        }

        let version = read_u32(&mut c)?;
        let instr_count = read_u32(&mut c)? as usize;
        let mut instructions = Vec::with_capacity(instr_count);
        for _ in 0..instr_count {
            instructions.push(decode_instruction(&mut c)?);
        }

        functions.insert(
            name.clone(),
            Function {
                name,
                params,
                instructions,
                version,
            },
        );
    }

    Ok(Program {
        functions,
        entry,
        state_layout,
    })
}

fn encode_instruction(out: &mut Vec<u8>, instruction: &Instruction) {
    match instruction {
        Instruction::LoadConst(v) => {
            out.push(0);
            encode_value(out, v);
        }
        Instruction::LoadVar(name) => {
            out.push(1);
            write_string(out, name);
        }
        Instruction::StoreVar(name) => {
            out.push(2);
            write_string(out, name);
        }
        Instruction::LoadGlobal(name) => {
            out.push(3);
            write_string(out, name);
        }
        Instruction::StoreGlobal(name) => {
            out.push(4);
            write_string(out, name);
        }
        Instruction::Add => out.push(5),
        Instruction::Sub => out.push(6),
        Instruction::Mul => out.push(7),
        Instruction::Div => out.push(8),
        Instruction::Equals => out.push(9),
        Instruction::NotEquals => out.push(10),
        Instruction::LessThan => out.push(11),
        Instruction::GreaterThan => out.push(12),
        Instruction::LessOrEqual => out.push(13),
        Instruction::GreaterOrEqual => out.push(14),
        Instruction::And => out.push(15),
        Instruction::Or => out.push(16),
        Instruction::Pop => out.push(17),
        Instruction::Jump(idx) => {
            out.push(18);
            write_u32(out, *idx as u32);
        }
        Instruction::JumpIfFalse(idx) => {
            out.push(19);
            write_u32(out, *idx as u32);
        }
        Instruction::CallFunction { name, arg_count } => {
            out.push(20);
            write_string(out, name);
            write_u16(out, *arg_count as u16);
        }
        Instruction::Return => out.push(21),
        Instruction::BuiltinPrint => out.push(22),
        Instruction::AllocObject { type_name } => {
            out.push(23);
            write_string(out, type_name);
        }
        Instruction::GetProp { name } => {
            out.push(24);
            write_string(out, name);
        }
        Instruction::SetProp { name } => {
            out.push(25);
            write_string(out, name);
        }
        Instruction::Retain => out.push(26),
        Instruction::Release => out.push(27),
        Instruction::CallNative { selector, arg_count } => {
            out.push(28);
            write_string(out, selector);
            write_u16(out, *arg_count as u16);
        }
    }
}

fn decode_instruction(c: &mut Cursor<&[u8]>) -> Result<Instruction, BytecodeError> {
    let opcode = read_u8(c)?;
    let instr = match opcode {
        0 => Instruction::LoadConst(decode_value(c)?),
        1 => Instruction::LoadVar(read_string(c)?),
        2 => Instruction::StoreVar(read_string(c)?),
        3 => Instruction::LoadGlobal(read_string(c)?),
        4 => Instruction::StoreGlobal(read_string(c)?),
        5 => Instruction::Add,
        6 => Instruction::Sub,
        7 => Instruction::Mul,
        8 => Instruction::Div,
        9 => Instruction::Equals,
        10 => Instruction::NotEquals,
        11 => Instruction::LessThan,
        12 => Instruction::GreaterThan,
        13 => Instruction::LessOrEqual,
        14 => Instruction::GreaterOrEqual,
        15 => Instruction::And,
        16 => Instruction::Or,
        17 => Instruction::Pop,
        18 => Instruction::Jump(read_u32(c)? as usize),
        19 => Instruction::JumpIfFalse(read_u32(c)? as usize),
        20 => Instruction::CallFunction {
            name: read_string(c)?,
            arg_count: read_u16(c)? as usize,
        },
        21 => Instruction::Return,
        22 => Instruction::BuiltinPrint,
        23 => Instruction::AllocObject {
            type_name: read_string(c)?,
        },
        24 => Instruction::GetProp {
            name: read_string(c)?,
        },
        25 => Instruction::SetProp {
            name: read_string(c)?,
        },
        26 => Instruction::Retain,
        27 => Instruction::Release,
        28 => Instruction::CallNative {
            selector: read_string(c)?,
            arg_count: read_u16(c)? as usize,
        },
        other => {
            return Err(BytecodeError::Malformed(format!(
                "unknown opcode {other}"
            )))
        }
    };
    Ok(instr)
}

fn encode_value(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Int(v) => {
            out.push(0);
            out.extend_from_slice(&v.to_le_bytes());
        }
        Value::Bool(v) => {
            out.push(1);
            out.push(if *v { 1 } else { 0 });
        }
        Value::Str(v) => {
            out.push(2);
            write_string(out, v);
        }
        Value::Nil => out.push(3),
        Value::Object(id) => {
            out.push(4);
            out.extend_from_slice(&id.to_le_bytes());
        }
    }
}

fn decode_value(c: &mut Cursor<&[u8]>) -> Result<Value, BytecodeError> {
    match read_u8(c)? {
        0 => {
            let mut buf = [0u8; 8];
            c.read_exact(&mut buf)
                .map_err(|_| BytecodeError::Malformed("missing i64 constant bytes".to_string()))?;
            Ok(Value::Int(i64::from_le_bytes(buf)))
        }
        1 => Ok(Value::Bool(read_u8(c)? != 0)),
        2 => Ok(Value::Str(read_string(c)?)),
        3 => Ok(Value::Nil),
        4 => {
            let mut buf = [0u8; 8];
            c.read_exact(&mut buf)
                .map_err(|_| BytecodeError::Malformed("missing object id bytes".to_string()))?;
            Ok(Value::Object(u64::from_le_bytes(buf)))
        }
        other => Err(BytecodeError::Malformed(format!(
            "unknown constant tag {other}"
        ))),
    }
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn read_u8(c: &mut Cursor<&[u8]>) -> Result<u8, BytecodeError> {
    let mut buf = [0u8; 1];
    c.read_exact(&mut buf)
        .map_err(|_| BytecodeError::Malformed("unexpected EOF reading u8".to_string()))?;
    Ok(buf[0])
}

fn read_u16(c: &mut Cursor<&[u8]>) -> Result<u16, BytecodeError> {
    let mut buf = [0u8; 2];
    c.read_exact(&mut buf)
        .map_err(|_| BytecodeError::Malformed("unexpected EOF reading u16".to_string()))?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32(c: &mut Cursor<&[u8]>) -> Result<u32, BytecodeError> {
    let mut buf = [0u8; 4];
    c.read_exact(&mut buf)
        .map_err(|_| BytecodeError::Malformed("unexpected EOF reading u32".to_string()))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_string(c: &mut Cursor<&[u8]>) -> Result<String, BytecodeError> {
    let len = read_u32(c)? as usize;
    let mut buf = vec![0u8; len];
    c.read_exact(&mut buf)
        .map_err(|_| BytecodeError::Malformed("unexpected EOF reading string".to_string()))?;
    String::from_utf8(buf).map_err(|_| BytecodeError::Malformed("invalid UTF-8 string".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_program() {
        let mut program = Program::with_entry("main");
        program.upsert_state_field(StateField {
            name: "counter".into(),
            type_name: "Int".into(),
            default_value: Some(Value::Int(0)),
        });
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::AllocObject {
                    type_name: "User".into(),
                },
                Instruction::StoreGlobal("x".into()),
                Instruction::LoadGlobal("x".into()),
                Instruction::Return,
            ],
        ));

        let bytes = encode_program(&program);
        let decoded = decode_program(&bytes).unwrap();
        assert_eq!(decoded.entry, "main");
        assert_eq!(decoded.functions["main"].instructions.len(), 4);
        assert_eq!(decoded.state_layout["counter"].type_name, "Int");
    }
}
