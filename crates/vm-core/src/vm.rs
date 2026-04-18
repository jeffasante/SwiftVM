use crate::errors::VMError;
use crate::heap::ArcHeap;
use crate::instructions::{Function, Instruction, Program};
use crate::value::Value;
use ffi_bridge::{call_native_rust, swiftvm_string_free, NativeTag, NativeValue};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Frame {
    function_name: String,
    pc: usize,
    locals: HashMap<String, Value>,
}

impl Frame {
    fn new(function: &Function, args: Vec<Value>) -> Result<Self, VMError> {
        if args.len() != function.params.len() {
            return Err(VMError::ArityMismatch {
                name: function.name.clone(),
                expected: function.params.len(),
                got: args.len(),
            });
        }

        let mut locals = HashMap::new();
        for (param, arg) in function.params.iter().zip(args.into_iter()) {
            locals.insert(param.clone(), arg);
        }

        Ok(Self {
            function_name: function.name.clone(),
            pc: 0,
            locals,
        })
    }
}

pub struct VM {
    stack: Vec<Value>,
    call_stack: Vec<Frame>,
    globals: HashMap<String, Value>,
    heap: ArcHeap,
    native_handlers: HashMap<String, NativeHandler>,
}

pub type NativeHandler = Arc<dyn Fn(&[Value]) -> Result<Value, VMError> + Send + Sync + 'static>;

impl Default for VM {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            call_stack: Vec::new(),
            globals: HashMap::new(),
            heap: ArcHeap::default(),
            native_handlers: HashMap::new(),
        }
    }
}

impl VM {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_runtime_state(&mut self) {
        self.stack.clear();
        self.call_stack.clear();
        self.globals.clear();
        self.heap = ArcHeap::default();
    }

    pub fn globals(&self) -> &HashMap<String, Value> {
        &self.globals
    }

    pub fn heap(&self) -> &ArcHeap {
        &self.heap
    }

    pub fn update_global(&mut self, name: &str, value: Value) {
        self.globals.insert(name.to_string(), value);
    }

    pub fn register_native_handler<F>(&mut self, selector: impl Into<String>, handler: F)
    where
        F: Fn(&[Value]) -> Result<Value, VMError> + Send + Sync + 'static,
    {
        self.native_handlers.insert(selector.into(), Arc::new(handler));
    }

    pub fn run_program(&mut self, program: &Program) -> Result<Value, VMError> {
        self.reset_runtime_state();
        self.apply_state_defaults(program);
        self.execute_function(program, &program.entry, vec![])
    }

    pub fn initialize_program_state(&mut self, program: &Program) {
        self.apply_state_defaults(program);
    }

    pub fn run_function(&mut self, program: &Program, function_name: &str) -> Result<Value, VMError> {
        self.stack.clear();
        self.call_stack.clear();
        self.execute_function(program, function_name, vec![])
    }

    fn apply_state_defaults(&mut self, program: &Program) {
        for (name, field) in &program.state_layout {
            if !self.globals.contains_key(name) {
                self.globals
                    .insert(name.clone(), field.default_value.clone().unwrap_or(Value::Nil));
            }
        }
    }

    fn execute_function(
        &mut self,
        program: &Program,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, VMError> {
        let entry = program
            .functions
            .get(function_name)
            .ok_or_else(|| VMError::FunctionNotFound(function_name.to_owned()))?;

        self.call_stack.push(Frame::new(entry, args)?);

        loop {
            let (frame_function_name, pc) = {
                let frame = self
                    .call_stack
                    .last()
                    .expect("frame must exist during execution");
                (frame.function_name.clone(), frame.pc)
            };

            let function = program
                .functions
                .get(&frame_function_name)
                .ok_or_else(|| VMError::FunctionNotFound(frame_function_name.clone()))?;

            if pc >= function.instructions.len() {
                return Err(VMError::InstructionOutOfRange {
                    function: frame_function_name,
                    pc,
                });
            }

            let instr = function.instructions[pc].clone();
            self.call_stack.last_mut().expect("frame exists").pc += 1;

            match instr {
                Instruction::LoadConst(value) => self.stack.push(value),
                Instruction::LoadVar(name) => {
                    let frame = self.call_stack.last().expect("frame exists");
                    let value = frame
                        .locals
                        .get(&name)
                        .cloned()
                        .ok_or_else(|| VMError::UndefinedVariable { name: name.clone() })?;
                    self.stack.push(value);
                }
                Instruction::StoreVar(name) => {
                    let value = self.pop("store var")?;
                    let frame = self.call_stack.last_mut().expect("frame exists");
                    frame.locals.insert(name, value);
                }
                Instruction::LoadGlobal(name) => {
                    let value = self.globals.get(&name).cloned().unwrap_or(Value::Nil);
                    self.stack.push(value);
                }
                Instruction::StoreGlobal(name) => {
                    let value = self.pop("store global")?;
                    self.globals.insert(name, value);
                }
                Instruction::Add => {
                    let right = self.pop("add")?;
                    let left = self.pop("add")?;

                    match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => {
                            self.stack.push(Value::Int(a + b));
                        }
                        (Value::Str(a), Value::Str(b)) => {
                            self.stack.push(Value::Str(format!("{}{}", a, b)));
                        }
                        (Value::Str(a), other) => {
                            self.stack.push(Value::Str(format!("{}{}", a, other)));
                        }
                        (other, Value::Str(b)) => {
                            self.stack.push(Value::Str(format!("{}{}", other, b)));
                        }
                        _ => {
                            return Err(VMError::TypeError(format!(
                                "add expected Int or String, got {:?} and {:?}",
                                left, right
                            )));
                        }
                    }
                }
                Instruction::Sub => {
                    let (a, b) = self.pop_int_pair("sub")?;
                    self.stack.push(Value::Int(a - b));
                }
                Instruction::Mul => {
                    let (a, b) = self.pop_int_pair("mul")?;
                    self.stack.push(Value::Int(a * b));
                }
                Instruction::Div => {
                    let (a, b) = self.pop_int_pair("div")?;
                    self.stack.push(Value::Int(a / b));
                }
                Instruction::Equals => {
                    let right = self.pop("equals")?;
                    let left = self.pop("equals")?;
                    self.stack.push(Value::Bool(left == right));
                }
                Instruction::NotEquals => {
                    let right = self.pop("not-equals")?;
                    let left = self.pop("not-equals")?;
                    self.stack.push(Value::Bool(left != right));
                }
                Instruction::LessThan => {
                    let (a, b) = self.pop_int_pair("less-than")?;
                    self.stack.push(Value::Bool(a < b));
                }
                Instruction::GreaterThan => {
                    let (a, b) = self.pop_int_pair("greater-than")?;
                    self.stack.push(Value::Bool(a > b));
                }
                Instruction::LessOrEqual => {
                    let (a, b) = self.pop_int_pair("less-or-equal")?;
                    self.stack.push(Value::Bool(a <= b));
                }
                Instruction::GreaterOrEqual => {
                    let (a, b) = self.pop_int_pair("greater-or-equal")?;
                    self.stack.push(Value::Bool(a >= b));
                }
                Instruction::And => {
                    let right = self.pop("and")?;
                    let left = self.pop("and")?;
                    self.stack.push(Value::Bool(left.is_truthy() && right.is_truthy()));
                }
                Instruction::Or => {
                    let right = self.pop("or")?;
                    let left = self.pop("or")?;
                    self.stack.push(Value::Bool(left.is_truthy() || right.is_truthy()));
                }
                Instruction::Pop => {
                    let _ = self.pop("pop")?;
                }
                Instruction::Jump(target) => {
                    self.validate_jump_target(target, function.instructions.len())?;
                    self.set_pc(target)?;
                }
                Instruction::JumpIfFalse(target) => {
                    let condition = self.pop("jump-if-false")?;
                    if !condition.is_truthy() {
                        self.validate_jump_target(target, function.instructions.len())?;
                        self.set_pc(target)?;
                    }
                }
                Instruction::CallFunction { name, arg_count } => {
                    if let Some(callee) = program.functions.get(&name) {
                        let mut call_args = Vec::with_capacity(arg_count);
                        for _ in 0..arg_count {
                            call_args.push(self.pop("call function arg")?);
                        }
                        call_args.reverse();
                        self.call_stack.push(Frame::new(callee, call_args)?);
                    } else {
                        // Built-in fallback: attempt to find a native handler
                        let mut args = Vec::with_capacity(arg_count);
                        for _ in 0..arg_count {
                            args.push(self.pop("builtin fallback arg")?);
                        }
                        args.reverse();

                        match self.call_native(&name, args) {
                            Ok(value) => self.stack.push(value),
                            Err(VMError::NativeSelectorNotFound { .. }) => {
                                return Err(VMError::FunctionNotFound(name.clone()));
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
                Instruction::AllocObject { type_name } => {
                    let id = self.heap.alloc(type_name);
                    self.stack.push(Value::Object(id));
                }
                Instruction::GetProp { name } => {
                    let obj_id = self
                        .pop("get prop")?
                        .as_object_id()
                        .ok_or_else(|| VMError::TypeError("get_prop expected object".to_string()))?;
                    let value = self
                        .heap
                        .get_prop(obj_id, &name)
                        .ok_or(VMError::ObjectNotFound { id: obj_id })?;
                    self.stack.push(value);
                }
                Instruction::SetProp { name } => {
                    let value = self.pop("set prop value")?;
                    let obj_id = self
                        .pop("set prop object")?
                        .as_object_id()
                        .ok_or_else(|| VMError::TypeError("set_prop expected object".to_string()))?;
                    if !self.heap.set_prop(obj_id, name, value) {
                        return Err(VMError::ObjectNotFound { id: obj_id });
                    }
                    self.stack.push(Value::Object(obj_id));
                }
                Instruction::Retain => {
                    let obj_id = self
                        .pop("retain")?
                        .as_object_id()
                        .ok_or_else(|| VMError::TypeError("retain expected object".to_string()))?;
                    if !self.heap.retain(obj_id) {
                        return Err(VMError::ObjectNotFound { id: obj_id });
                    }
                    self.stack.push(Value::Object(obj_id));
                }
                Instruction::Release => {
                    let obj_id = self
                        .pop("release")?
                        .as_object_id()
                        .ok_or_else(|| VMError::TypeError("release expected object".to_string()))?;
                    if !self.heap.release(obj_id) {
                        return Err(VMError::ObjectNotFound { id: obj_id });
                    }
                    self.stack.push(Value::Nil);
                }
                Instruction::CallNative { selector, arg_count } => {
                    let mut args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        args.push(self.pop("native call arg")?);
                    }
                    args.reverse();
                    let value = self.call_native(&selector, args)?;
                    self.stack.push(value);
                }
                Instruction::BuiltinPrint => {
                    let value = self.pop("print")?;
                    println!("{value}");
                    self.stack.push(Value::Nil);
                }
                Instruction::Return => {
                    let ret = self.stack.pop().unwrap_or(Value::Nil);
                    self.call_stack.pop();

                    if self.call_stack.is_empty() {
                        return Ok(ret);
                    }

                    self.stack.push(ret);
                }
            }
        }
    }

    fn call_native(&self, selector: &str, args: Vec<Value>) -> Result<Value, VMError> {
        if let Some(handler) = self.native_handlers.get(selector) {
            return handler(&args);
        }

        let mut native_args = Vec::with_capacity(args.len());
        let mut owned_arg_strings: Vec<*mut c_char> = Vec::new();
        for arg in &args {
            let (native, owned) = Self::vm_value_to_native(arg)?;
            if let Some(ptr) = owned {
                owned_arg_strings.push(ptr);
            }
            native_args.push(native);
        }

        let ffi_result = call_native_rust(selector, &native_args);

        for ptr in owned_arg_strings {
            unsafe { swiftvm_string_free(ptr) };
        }

        match ffi_result {
            Ok(native) => return Self::native_to_vm_value(native),
            Err(-4) => {}
            Err(code) => {
                return Err(VMError::TypeError(format!(
                    "native call failed for `{selector}` with code {code}"
                )))
            }
        }

        match selector {
            "debug.echo" => Ok(args.into_iter().next().unwrap_or(Value::Nil)),
            "debug.sum2" => {
                let a = args.first().and_then(Value::as_int).unwrap_or(0);
                let b = args.get(1).and_then(Value::as_int).unwrap_or(0);
                Ok(Value::Int(a + b))
            }
            _ => Err(VMError::NativeSelectorNotFound {
                selector: selector.to_string(),
            }),
        }
    }

    fn vm_value_to_native(value: &Value) -> Result<(NativeValue, Option<*mut c_char>), VMError> {
        match value {
            Value::Int(v) => Ok((NativeValue::int(*v), None)),
            Value::Bool(v) => Ok((NativeValue::bool(*v), None)),
            Value::Str(v) => {
                let native = NativeValue::string(v);
                Ok((native, Some(native.string_ptr as *mut c_char)))
            }
            Value::Nil => Ok((NativeValue::nil(), None)),
            Value::Object(_) => Err(VMError::TypeError(
                "native call does not yet support Object arguments".to_string(),
            )),
        }
    }

    fn native_to_vm_value(native: NativeValue) -> Result<Value, VMError> {
        match native.tag {
            NativeTag::Int => Ok(Value::Int(native.int_value)),
            NativeTag::Bool => Ok(Value::Bool(native.bool_value != 0)),
            NativeTag::Nil => Ok(Value::Nil),
            NativeTag::String => {
                if native.string_ptr.is_null() {
                    return Ok(Value::Str(String::new()));
                }
                let text = unsafe { CStr::from_ptr(native.string_ptr) }
                    .to_string_lossy()
                    .into_owned();
                unsafe { swiftvm_string_free(native.string_ptr as *mut c_char) };
                Ok(Value::Str(text))
            }
        }
    }

    fn set_pc(&mut self, target: usize) -> Result<(), VMError> {
        let frame = self.call_stack.last_mut().expect("frame exists");
        frame.pc = target;
        Ok(())
    }

    fn validate_jump_target(&self, target: usize, instruction_len: usize) -> Result<(), VMError> {
        if target >= instruction_len {
            return Err(VMError::InvalidJumpTarget { target });
        }
        Ok(())
    }

    fn pop(&mut self, op: &'static str) -> Result<Value, VMError> {
        self.stack.pop().ok_or(VMError::StackUnderflow { op })
    }

    fn pop_int_pair(&mut self, op: &'static str) -> Result<(i64, i64), VMError> {
        let right = self.pop(op)?;
        let left = self.pop(op)?;

        let right_int = right
            .as_int()
            .ok_or_else(|| VMError::TypeError(format!("{op} expected Int on right operand")))?;
        let left_int = left
            .as_int()
            .ok_or_else(|| VMError::TypeError(format!("{op} expected Int on left operand")))?;

        Ok((left_int, right_int))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffi_bridge::{register_native_rust, NativeTag, NativeValue};
    use std::ffi::CStr;
    use crate::instructions::{Function, Instruction, Program, StateField};

    extern "C" fn ffi_sum2(args: *const NativeValue, arg_count: usize, out_result: *mut NativeValue) -> i32 {
        if args.is_null() || out_result.is_null() || arg_count < 2 {
            return -9;
        }
        let slice = unsafe { std::slice::from_raw_parts(args, arg_count) };
        if slice[0].tag != NativeTag::Int || slice[1].tag != NativeTag::Int {
            return -8;
        }
        unsafe {
            *out_result = NativeValue::int(slice[0].int_value + slice[1].int_value);
        }
        0
    }

    extern "C" fn ffi_uppercase(
        args: *const NativeValue,
        arg_count: usize,
        out_result: *mut NativeValue,
    ) -> i32 {
        if args.is_null() || out_result.is_null() || arg_count < 1 {
            return -9;
        }
        let slice = unsafe { std::slice::from_raw_parts(args, arg_count) };
        if slice[0].tag != NativeTag::String || slice[0].string_ptr.is_null() {
            return -8;
        }
        let input = unsafe { CStr::from_ptr(slice[0].string_ptr) }
            .to_string_lossy()
            .to_uppercase();
        unsafe {
            *out_result = NativeValue::string(&input);
        }
        0
    }

    #[test]
    fn executes_function_calls_and_arithmetic() {
        let mut program = Program::with_entry("main");

        program.add_function(Function::new(
            "add",
            vec!["a", "b"],
            vec![
                Instruction::LoadVar("a".into()),
                Instruction::LoadVar("b".into()),
                Instruction::Add,
                Instruction::Return,
            ],
        ));

        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::LoadConst(Value::Int(2)),
                Instruction::LoadConst(Value::Int(3)),
                Instruction::CallFunction {
                    name: "add".into(),
                    arg_count: 2,
                },
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Int(5));
    }

    #[test]
    fn executes_conditional_jumps() {
        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::LoadConst(Value::Bool(false)),
                Instruction::JumpIfFalse(4),
                Instruction::LoadConst(Value::Int(99)),
                Instruction::Return,
                Instruction::LoadConst(Value::Int(42)),
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn preserves_globals_across_run_function_calls() {
        let mut program = Program::with_entry("init");

        program.add_function(Function::new(
            "init",
            vec![],
            vec![
                Instruction::LoadConst(Value::Int(0)),
                Instruction::StoreGlobal("counter".into()),
                Instruction::LoadConst(Value::Nil),
                Instruction::Return,
            ],
        ));

        program.add_function(Function::new(
            "tick",
            vec![],
            vec![
                Instruction::LoadGlobal("counter".into()),
                Instruction::LoadConst(Value::Int(1)),
                Instruction::Add,
                Instruction::StoreGlobal("counter".into()),
                Instruction::LoadGlobal("counter".into()),
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        vm.run_program(&program).unwrap();

        let one = vm.run_function(&program, "tick").unwrap();
        let two = vm.run_function(&program, "tick").unwrap();

        assert_eq!(one, Value::Int(1));
        assert_eq!(two, Value::Int(2));
    }

    #[test]
    fn allocates_and_updates_heap_object() {
        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::AllocObject {
                    type_name: "User".into(),
                },
                Instruction::LoadConst(Value::Int(42)),
                Instruction::SetProp {
                    name: "age".into(),
                },
                Instruction::GetProp {
                    name: "age".into(),
                },
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn applies_state_defaults() {
        let mut program = Program::with_entry("main");
        program.upsert_state_field(StateField {
            name: "count".into(),
            type_name: "Int".into(),
            default_value: Some(Value::Int(9)),
        });
        program.add_function(Function::new(
            "main",
            vec![],
            vec![Instruction::LoadGlobal("count".into()), Instruction::Return],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Int(9));
    }

    #[test]
    fn executes_extended_comparisons_and_pop() {
        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::LoadConst(Value::Int(4)),
                Instruction::LoadConst(Value::Int(4)),
                Instruction::NotEquals,
                Instruction::Pop,
                Instruction::LoadConst(Value::Int(5)),
                Instruction::LoadConst(Value::Int(3)),
                Instruction::GreaterThan,
                Instruction::Pop,
                Instruction::LoadConst(Value::Int(5)),
                Instruction::LoadConst(Value::Int(5)),
                Instruction::LessOrEqual,
                Instruction::Pop,
                Instruction::LoadConst(Value::Int(6)),
                Instruction::LoadConst(Value::Int(6)),
                Instruction::GreaterOrEqual,
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Bool(true));
    }

    #[test]
    fn executes_logical_and_or() {
        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::LoadConst(Value::Bool(true)),
                Instruction::LoadConst(Value::Bool(false)),
                Instruction::Or,
                Instruction::LoadConst(Value::Bool(true)),
                Instruction::And,
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Bool(true));
    }

    #[test]
    fn rejects_out_of_bounds_jump_targets() {
        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![Instruction::Jump(999), Instruction::Return],
        ));

        let mut vm = VM::new();
        let err = vm.run_program(&program).unwrap_err();
        assert!(matches!(err, VMError::InvalidJumpTarget { target: 999 }));
    }

    #[test]
    fn executes_registered_native_handler() {
        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::LoadConst(Value::Int(20)),
                Instruction::LoadConst(Value::Int(22)),
                Instruction::CallNative {
                    selector: "math.sum2".into(),
                    arg_count: 2,
                },
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        vm.register_native_handler("math.sum2", |args| {
            let a = args.first().and_then(Value::as_int).unwrap_or(0);
            let b = args.get(1).and_then(Value::as_int).unwrap_or(0);
            Ok(Value::Int(a + b))
        });

        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn executes_registered_ffi_bridge_native_selector() {
        register_native_rust("ffi.math.sum2", ffi_sum2).unwrap();

        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::LoadConst(Value::Int(9)),
                Instruction::LoadConst(Value::Int(33)),
                Instruction::CallNative {
                    selector: "ffi.math.sum2".into(),
                    arg_count: 2,
                },
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn executes_registered_ffi_bridge_string_selector() {
        register_native_rust("ffi.str.upper", ffi_uppercase).unwrap();

        let mut program = Program::with_entry("main");
        program.add_function(Function::new(
            "main",
            vec![],
            vec![
                Instruction::LoadConst(Value::Str("swift vm".into())),
                Instruction::CallNative {
                    selector: "ffi.str.upper".into(),
                    arg_count: 1,
                },
                Instruction::Return,
            ],
        ));

        let mut vm = VM::new();
        let value = vm.run_program(&program).unwrap();
        assert_eq!(value, Value::Str("SWIFT VM".into()));
    }
}
