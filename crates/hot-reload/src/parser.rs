use std::str::FromStr;
use thiserror::Error;
use vm_core::{Function, Instruction, Program, StateField, Value};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("line {line}: {message}")]
    InvalidLine { line: usize, message: String },

    #[error("line {line}: function `{name}` was not closed with `end`")]
    UnterminatedFunction { line: usize, name: String },

    #[error("no `main` function found in source")]
    MissingMain,
}

pub fn parse_program_text(source: &str) -> Result<Program, ParseError> {
    let mut functions = Vec::<Function>::new();
    let mut state_fields = Vec::<StateField>::new();
    let mut current_name: Option<String> = None;
    let mut current_params = Vec::<String>::new();
    let mut current_instr = Vec::<Instruction>::new();
    let mut start_line = 0usize;

    for (idx, raw) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(name) = &current_name {
            if line == "end" {
                functions.push(Function::with_owned_params(
                    name.clone(),
                    std::mem::take(&mut current_params),
                    std::mem::take(&mut current_instr),
                ));
                current_name = None;
                continue;
            }

            current_instr.push(parse_instruction(line, line_no)?);
            continue;
        }

        if let Some(rest) = line.strip_prefix("state ") {
            state_fields.push(parse_state_line(rest, line_no)?);
            continue;
        }

        if let Some(header) = line.strip_prefix("func ") {
            let open_paren = header.find('(').ok_or_else(|| ParseError::InvalidLine {
                line: line_no,
                message: "function declaration must include parentheses".to_string(),
            })?;
            let close_paren = header.rfind(')').ok_or_else(|| ParseError::InvalidLine {
                line: line_no,
                message: "function declaration missing closing ')'".to_string(),
            })?;

            let name = header[..open_paren].trim();
            if name.is_empty() {
                return Err(ParseError::InvalidLine {
                    line: line_no,
                    message: "function name cannot be empty".to_string(),
                });
            }

            let params_blob = &header[open_paren + 1..close_paren];
            let params = if params_blob.trim().is_empty() {
                Vec::new()
            } else {
                params_blob
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .collect::<Vec<_>>()
            };

            current_name = Some(name.to_string());
            current_params = params;
            start_line = line_no;
            continue;
        }

        return Err(ParseError::InvalidLine {
            line: line_no,
            message: "expected `state ...` or `func ...` declaration".to_string(),
        });
    }

    if let Some(name) = current_name {
        return Err(ParseError::UnterminatedFunction {
            line: start_line,
            name,
        });
    }

    let mut program = Program::with_entry("main");
    for field in state_fields {
        program.upsert_state_field(field);
    }
    for function in functions {
        program.add_function(function);
    }

    if !program.functions.contains_key("main") {
        return Err(ParseError::MissingMain);
    }

    Ok(program)
}

fn parse_state_line(rest: &str, line_no: usize) -> Result<StateField, ParseError> {
    // format: state name:Type=default
    let (left, default_value) = if let Some(eq_idx) = rest.find('=') {
        let lhs = rest[..eq_idx].trim();
        let rhs = rest[eq_idx + 1..].trim();
        (lhs, Some(parse_value(rhs, line_no)?))
    } else {
        (rest.trim(), None)
    };

    let (name, type_name) = left
        .split_once(':')
        .ok_or_else(|| ParseError::InvalidLine {
            line: line_no,
            message: "state declaration must be name:Type with optional =default".to_string(),
        })?;

    let field_name = name.trim();
    let field_type = type_name.trim();
    if field_name.is_empty() || field_type.is_empty() {
        return Err(ParseError::InvalidLine {
            line: line_no,
            message: "state name and type cannot be empty".to_string(),
        });
    }

    Ok(StateField {
        name: field_name.to_string(),
        type_name: field_type.to_string(),
        default_value,
    })
}

fn parse_instruction(line: &str, line_no: usize) -> Result<Instruction, ParseError> {
    let mut parts = line.split_whitespace();
    let op = parts.next().ok_or_else(|| ParseError::InvalidLine {
        line: line_no,
        message: "empty instruction".to_string(),
    })?;

    match op {
        "load_const" => {
            let value_text = line["load_const".len()..].trim();
            let value = parse_value(value_text, line_no)?;
            Ok(Instruction::LoadConst(value))
        }
        "load_var" => Ok(Instruction::LoadVar(read_identifier(parts.next(), line_no, "load_var")?)),
        "store_var" => Ok(Instruction::StoreVar(read_identifier(parts.next(), line_no, "store_var")?)),
        "load_global" => Ok(Instruction::LoadGlobal(read_identifier(parts.next(), line_no, "load_global")?)),
        "store_global" => Ok(Instruction::StoreGlobal(read_identifier(parts.next(), line_no, "store_global")?)),
        "add" => Ok(Instruction::Add),
        "sub" => Ok(Instruction::Sub),
        "mul" => Ok(Instruction::Mul),
        "div" => Ok(Instruction::Div),
        "eq" => Ok(Instruction::Equals),
        "ne" => Ok(Instruction::NotEquals),
        "lt" => Ok(Instruction::LessThan),
        "gt" => Ok(Instruction::GreaterThan),
        "le" => Ok(Instruction::LessOrEqual),
        "ge" => Ok(Instruction::GreaterOrEqual),
        "and" => Ok(Instruction::And),
        "or" => Ok(Instruction::Or),
        "pop" => Ok(Instruction::Pop),
        "jump" => Ok(Instruction::Jump(read_usize(parts.next(), line_no, "jump")?)),
        "jump_if_false" => Ok(Instruction::JumpIfFalse(read_usize(
            parts.next(),
            line_no,
            "jump_if_false",
        )?)),
        "call" => {
            let name = read_identifier(parts.next(), line_no, "call")?;
            let argc = read_usize(parts.next(), line_no, "call")?;
            Ok(Instruction::CallFunction {
                name,
                arg_count: argc,
            })
        }
        "alloc_object" => Ok(Instruction::AllocObject {
            type_name: read_identifier(parts.next(), line_no, "alloc_object")?,
        }),
        "get_prop" => Ok(Instruction::GetProp {
            name: read_identifier(parts.next(), line_no, "get_prop")?,
        }),
        "set_prop" => Ok(Instruction::SetProp {
            name: read_identifier(parts.next(), line_no, "set_prop")?,
        }),
        "retain" => Ok(Instruction::Retain),
        "release" => Ok(Instruction::Release),
        "native_call" => {
            let selector = read_identifier(parts.next(), line_no, "native_call")?;
            let argc = read_usize(parts.next(), line_no, "native_call")?;
            Ok(Instruction::CallNative {
                selector,
                arg_count: argc,
            })
        }
        "print" => Ok(Instruction::BuiltinPrint),
        "return" => Ok(Instruction::Return),
        _ => Err(ParseError::InvalidLine {
            line: line_no,
            message: format!("unknown instruction `{op}`"),
        }),
    }
}

fn read_identifier(token: Option<&str>, line_no: usize, op: &str) -> Result<String, ParseError> {
    let token = token.ok_or_else(|| ParseError::InvalidLine {
        line: line_no,
        message: format!("{op} expects an identifier"),
    })?;

    if token.is_empty() {
        return Err(ParseError::InvalidLine {
            line: line_no,
            message: format!("{op} expects a non-empty identifier"),
        });
    }

    Ok(token.to_string())
}

fn read_usize(token: Option<&str>, line_no: usize, op: &str) -> Result<usize, ParseError> {
    let token = token.ok_or_else(|| ParseError::InvalidLine {
        line: line_no,
        message: format!("{op} expects a numeric argument"),
    })?;

    usize::from_str(token).map_err(|_| ParseError::InvalidLine {
        line: line_no,
        message: format!("{op} expects a valid usize"),
    })
}

fn parse_value(raw: &str, line_no: usize) -> Result<Value, ParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ParseError::InvalidLine {
            line: line_no,
            message: "value cannot be empty".to_string(),
        });
    }

    if trimmed == "nil" {
        return Ok(Value::Nil);
    }
    if trimmed == "true" {
        return Ok(Value::Bool(true));
    }
    if trimmed == "false" {
        return Ok(Value::Bool(false));
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        let inside = &trimmed[1..trimmed.len() - 1];
        return Ok(Value::Str(inside.to_string()));
    }
    if let Ok(v) = i64::from_str(trimmed) {
        return Ok(Value::Int(v));
    }

    Err(ParseError::InvalidLine {
        line: line_no,
        message: format!("unsupported literal `{trimmed}`"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_program() {
        let source = r#"
        state counter:Int=0
        func main()
          load_const 1
          load_const 2
          add
          return
        end
        "#;

        let program = parse_program_text(source).unwrap();
        let main = program.functions.get("main").unwrap();
        assert_eq!(main.instructions.len(), 4);
        assert_eq!(program.state_layout["counter"].type_name, "Int");
    }

    #[test]
    fn parses_globals_calls_and_object_ops() {
        let source = r#"
        state title:String="hello"

        func add(a,b)
          load_var a
          load_var b
          add
          return
        end

        func main()
          alloc_object User
          load_const 22
          set_prop age
          load_global counter
          load_const 2
          call add 2
          native_call debug.echo 1
          return
        end
        "#;

        let program = parse_program_text(source).unwrap();
        assert!(program.functions.contains_key("add"));
        assert!(program.functions.contains_key("main"));
        assert!(program.state_layout.contains_key("title"));
    }

    #[test]
    fn parses_logical_instructions() {
        let source = r#"
        func main()
          load_const true
          load_const false
          and
          load_const true
          or
          return
        end
        "#;

        let program = parse_program_text(source).unwrap();
        let main = program.functions.get("main").unwrap();
        assert!(matches!(main.instructions[2], Instruction::And));
        assert!(matches!(main.instructions[4], Instruction::Or));
    }
}
